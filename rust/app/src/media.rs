// SoundCloud sends a full song as fragmented MP4. The decoder plays a normal file, so AAC frames are wrapped in ADTS.

pub fn playable(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    if is_fragmented(&bytes) {
        return aac_adts(&bytes);
    }
    Ok(without_video(bytes))
}

/// A music video stores the picture first. The decoder then times the song as if those picture frames were audio, so a seek past that short mark jumps to the end.
fn without_video(bytes: Vec<u8>) -> Vec<u8> {
    audio_only(&bytes).unwrap_or(bytes)
}

fn audio_only(bytes: &[u8]) -> Option<Vec<u8>> {
    let boxes = top_level(bytes);
    let moov = boxes.iter().find(|item| item.kind == *b"moov")?;
    let mdat = boxes.iter().find(|item| item.kind == *b"mdat")?;
    let new_moov = moov_without_video(bytes, moov)?;
    if moov.start + moov.size <= mdat.start {
        if moov.start + new_moov.len() > mdat.start {
            return None;
        }
        let mut out = Vec::with_capacity(bytes.len());
        out.extend_from_slice(&bytes[..moov.start]);
        out.extend_from_slice(&new_moov);
        let pad = mdat.start - out.len();
        if pad > 0 {
            out.extend(free_box(pad)?);
        }
        out.extend_from_slice(&bytes[mdat.start..]);
        Some(out)
    } else if moov.start >= mdat.start + mdat.size {
        let mut out = Vec::with_capacity(bytes.len() - moov.size + new_moov.len());
        out.extend_from_slice(&bytes[..moov.start]);
        out.extend_from_slice(&new_moov);
        out.extend_from_slice(&bytes[moov.start + moov.size..]);
        Some(out)
    } else {
        None
    }
}

fn moov_without_video(bytes: &[u8], moov: &Mp4Box) -> Option<Vec<u8>> {
    let mut payload = Vec::new();
    let mut removed = false;
    let mut kept_audio = false;
    for child in children(bytes, moov, 0) {
        if child.kind == *b"trak" {
            match handler(bytes, &child) {
                Some(kind) if kind == *b"vide" => {
                    removed = true;
                    continue;
                }
                Some(kind) if kind == *b"soun" => kept_audio = true,
                _ => {}
            }
        }
        payload.extend_from_slice(&bytes[child.start..child.start + child.size]);
    }
    if !removed || !kept_audio {
        return None;
    }
    make_box(b"moov", &payload)
}

fn handler(bytes: &[u8], trak: &Mp4Box) -> Option<[u8; 4]> {
    let hdlr = find_nested(bytes, trak, *b"hdlr")?;
    let cursor = hdlr.start + hdlr.header + 8;
    bytes.get(cursor..cursor + 4)?.try_into().ok()
}

fn make_box(kind: &[u8; 4], payload: &[u8]) -> Option<Vec<u8>> {
    let size = u32::try_from(payload.len().checked_add(8)?).ok()?;
    let mut out = Vec::with_capacity(size as usize);
    out.extend_from_slice(&size.to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(payload);
    Some(out)
}

fn free_box(size: usize) -> Option<Vec<u8>> {
    if size < 8 {
        return None;
    }
    let declared = u32::try_from(size).ok()?;
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(&declared.to_be_bytes());
    out.extend_from_slice(b"free");
    out.resize(size, 0);
    Some(out)
}

/// Length of a whole song file, in seconds. YouTube Music often omits this, so the player reads it from the file it is about to play.
pub fn length_seconds(bytes: &[u8]) -> Option<u32> {
    movie_seconds(bytes).or_else(|| adts_seconds(bytes))
}

fn is_fragmented(bytes: &[u8]) -> bool {
    top_level(bytes).iter().any(|item| item.kind == *b"moof")
}

fn aac_adts(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let (track_id, asc) = audio_config(bytes)?;
    let object_type = (asc[0] >> 3) & 0x1f;
    let rate_index = ((asc[0] & 0x07) << 1) | (asc[1] >> 7);
    if object_type == 0 || object_type > 4 || rate_index == 0x0f {
        return Err(undecodable());
    }
    let mut output = Vec::with_capacity(bytes.len() + bytes.len() / 40);
    let mut frames = 0usize;
    for item in top_level(bytes) {
        if item.kind != *b"moof" {
            continue;
        }
        for (start, size) in fragment_samples(bytes, &item, track_id)? {
            let end = start.checked_add(size).ok_or_else(undecodable)?;
            if end > bytes.len() || size == 0 {
                return Err(undecodable());
            }
            output.extend_from_slice(&adts_header(&asc, size));
            output.extend_from_slice(&bytes[start..end]);
            frames += 1;
        }
    }
    if frames == 0 {
        return Err(undecodable());
    }
    Ok(output)
}

fn undecodable() -> String {
    "This song is a stream the native player cannot decode yet.".into()
}

fn audio_config(bytes: &[u8]) -> Result<(u32, Vec<u8>), String> {
    let moov = top_level(bytes).into_iter().find(|item| item.kind == *b"moov").ok_or_else(undecodable)?;
    for trak in children(bytes, &moov, 0) {
        if trak.kind != *b"trak" {
            continue;
        }
        let Some(track_id) = track_id(bytes, &trak) else { continue };
        let Some(mp4a) = find_nested(bytes, &trak, *b"mp4a") else { continue };
        let Some(esds) = children(bytes, &mp4a, 28).into_iter().find(|item| item.kind == *b"esds") else { continue };
        if let Some(asc) = audio_specific_config(bytes, &esds) {
            return Ok((track_id, asc));
        }
    }
    Err(undecodable())
}

fn track_id(bytes: &[u8], trak: &Mp4Box) -> Option<u32> {
    let tkhd = children(bytes, trak, 0).into_iter().find(|item| item.kind == *b"tkhd")?;
    let (version, _, mut cursor) = full_box(bytes, &tkhd).ok()?;
    if version == 1 {
        cursor = cursor.checked_add(16)?;
    } else {
        cursor = cursor.checked_add(8)?;
    }
    read_u32(bytes, cursor)
}

fn find_nested(bytes: &[u8], parent: &Mp4Box, kind: [u8; 4]) -> Option<Mp4Box> {
    let mut pending = children(bytes, parent, 0);
    while let Some(item) = pending.pop() {
        if item.kind == kind {
            return Some(item);
        }
        if matches!(&item.kind, b"mdia" | b"minf" | b"stbl" | b"stsd") {
            let extra = if item.kind == *b"stsd" { 8 } else { 0 };
            pending.extend(children(bytes, &item, extra));
        }
    }
    None
}

fn audio_specific_config(bytes: &[u8], esds: &Mp4Box) -> Option<Vec<u8>> {
    let (_, _, mut cursor) = full_box(bytes, esds).ok()?;
    if *bytes.get(cursor)? != 0x03 {
        return None;
    }
    cursor += 1;
    let (es_len, next) = expandable(bytes, cursor)?;
    cursor = next;
    let es_end = cursor.checked_add(es_len)?;
    cursor = cursor.checked_add(2)?;
    let flags = *bytes.get(cursor)?;
    cursor += 1;
    if flags & 0x80 != 0 {
        let length = *bytes.get(cursor)? as usize;
        cursor = cursor.checked_add(1 + length)?;
    }
    if flags & 0x40 != 0 {
        let length = *bytes.get(cursor)? as usize;
        cursor = cursor.checked_add(1 + length)?;
    }
    if flags & 0x20 != 0 {
        cursor = cursor.checked_add(2)?;
    }
    if *bytes.get(cursor)? != 0x04 || cursor >= es_end {
        return None;
    }
    cursor += 1;
    let (_, next) = expandable(bytes, cursor)?;
    cursor = next;
    if *bytes.get(cursor)? != 0x40 {
        return None;
    }
    cursor = cursor.checked_add(1 + 1 + 3 + 4 + 4)?;
    if *bytes.get(cursor)? != 0x05 {
        return None;
    }
    cursor += 1;
    let (length, next) = expandable(bytes, cursor)?;
    cursor = next;
    if length < 2 || cursor + length > bytes.len() {
        return None;
    }
    Some(bytes[cursor..cursor + length].to_vec())
}

fn fragment_samples(bytes: &[u8], moof: &Mp4Box, track_id: u32) -> Result<Vec<(usize, usize)>, String> {
    let mut samples = Vec::new();
    for traf in children(bytes, moof, 0) {
        if traf.kind != *b"traf" {
            continue;
        }
        let Some(tfhd) = children(bytes, &traf, 0).into_iter().find(|child| child.kind == *b"tfhd") else { continue };
        let parsed = parse_tfhd(bytes, &tfhd, moof.start)?;
        if parsed.track_id != track_id {
            continue;
        }
        for trun in children(bytes, &traf, 0) {
            if trun.kind == *b"trun" {
                samples.extend(parse_trun(bytes, &trun, parsed.base, parsed.default_size)?);
            }
        }
    }
    Ok(samples)
}

struct Tfhd {
    track_id: u32,
    base: u64,
    default_size: u32,
}

fn parse_tfhd(bytes: &[u8], tfhd: &Mp4Box, moof_start: usize) -> Result<Tfhd, String> {
    let (_, flags, mut cursor) = full_box(bytes, tfhd)?;
    let track_id = read_u32(bytes, cursor).ok_or_else(undecodable)?;
    cursor += 4;
    let mut base = moof_start as u64;
    if flags & 0x0000_0001 != 0 {
        base = read_u64(bytes, cursor).ok_or_else(undecodable)?;
        cursor += 8;
    }
    if flags & 0x0000_0002 != 0 {
        cursor += 4;
    }
    if flags & 0x0000_0008 != 0 {
        cursor += 4;
    }
    let mut default_size = 0u32;
    if flags & 0x0000_0010 != 0 {
        default_size = read_u32(bytes, cursor).ok_or_else(undecodable)?;
    }
    Ok(Tfhd { track_id, base, default_size })
}

fn parse_trun(bytes: &[u8], trun: &Mp4Box, base: u64, default_size: u32) -> Result<Vec<(usize, usize)>, String> {
    let (_, flags, mut cursor) = full_box(bytes, trun)?;
    let count = read_u32(bytes, cursor).ok_or_else(undecodable)? as usize;
    cursor += 4;
    if count > 100_000 {
        return Err(undecodable());
    }
    let mut data_offset = 0i32;
    if flags & 0x0000_0001 != 0 {
        data_offset = read_i32(bytes, cursor).ok_or_else(undecodable)?;
        cursor += 4;
    }
    if flags & 0x0000_0004 != 0 {
        cursor += 4;
    }
    let mut start = i64::try_from(base).unwrap_or(0) + i64::from(data_offset);
    let mut samples = Vec::with_capacity(count);
    for _ in 0..count {
        if flags & 0x0000_0100 != 0 {
            cursor += 4;
        }
        let size = if flags & 0x0000_0200 != 0 {
            let value = read_u32(bytes, cursor).ok_or_else(undecodable)?;
            cursor += 4;
            value
        } else {
            default_size
        };
        if flags & 0x0000_0400 != 0 {
            cursor += 4;
        }
        if flags & 0x0000_0800 != 0 {
            cursor += 4;
        }
        if size == 0 {
            continue;
        }
        if start < 0 {
            return Err(undecodable());
        }
        samples.push((start as usize, size as usize));
        start += i64::from(size);
    }
    Ok(samples)
}

fn adts_header(asc: &[u8], frame_len: usize) -> [u8; 7] {
    let aot = (asc[0] >> 3) & 0x1f;
    let freq = ((asc[0] & 0x07) << 1) | (asc[1] >> 7);
    let channels = (asc[1] >> 3) & 0x0f;
    let profile = aot.saturating_sub(1);
    let full = frame_len + 7;
    [
        0xFF,
        0xF1,
        (profile << 6) | (freq << 2) | ((channels >> 2) & 0x01),
        ((channels & 0x03) << 6) | ((full >> 11) & 0x03) as u8,
        ((full >> 3) & 0xFF) as u8,
        (((full & 0x07) << 5) as u8) | 0x1F,
        0xFC,
    ]
}

struct Mp4Box {
    start: usize,
    size: usize,
    header: usize,
    kind: [u8; 4],
}

fn top_level(bytes: &[u8]) -> Vec<Mp4Box> {
    let mut items = Vec::new();
    let mut pos = 0;
    while pos + 8 <= bytes.len() {
        let Some(item) = read_box(bytes, pos) else { break };
        pos = item.start + item.size;
        items.push(item);
    }
    items
}

fn children(bytes: &[u8], parent: &Mp4Box, extra: usize) -> Vec<Mp4Box> {
    let mut items = Vec::new();
    let end = parent.start + parent.size;
    let mut pos = parent.start + parent.header + extra;
    while pos + 8 <= end {
        let Some(item) = read_box(bytes, pos) else { break };
        if item.start + item.size > end {
            break;
        }
        pos = item.start + item.size;
        items.push(item);
    }
    items
}

fn read_box(bytes: &[u8], pos: usize) -> Option<Mp4Box> {
    if pos + 8 > bytes.len() {
        return None;
    }
    let mut size = u32::from_be_bytes(bytes[pos..pos + 4].try_into().ok()?) as usize;
    let kind = bytes[pos + 4..pos + 8].try_into().ok()?;
    let mut header = 8usize;
    if size == 1 {
        if pos + 16 > bytes.len() {
            return None;
        }
        size = u64::from_be_bytes(bytes[pos + 8..pos + 16].try_into().ok()?) as usize;
        header = 16;
    }
    if size < header || pos + size > bytes.len() {
        return None;
    }
    Some(Mp4Box { start: pos, size, header, kind })
}

fn full_box(bytes: &[u8], item: &Mp4Box) -> Result<(u8, u32, usize), String> {
    let cursor = item.start + item.header;
    if cursor + 4 > bytes.len() {
        return Err(undecodable());
    }
    let word = u32::from_be_bytes(bytes[cursor..cursor + 4].try_into().map_err(|_| undecodable())?);
    Ok(((word >> 24) as u8, word & 0x00FF_FFFF, cursor + 4))
}

fn expandable(bytes: &[u8], mut cursor: usize) -> Option<(usize, usize)> {
    let mut value = 0usize;
    for _ in 0..4 {
        let byte = *bytes.get(cursor)?;
        cursor += 1;
        value = (value << 7) | (byte & 0x7f) as usize;
        if byte & 0x80 == 0 {
            return Some((value, cursor));
        }
    }
    None
}

fn read_u32(bytes: &[u8], cursor: usize) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.get(cursor..cursor + 4)?.try_into().ok()?))
}

fn read_u64(bytes: &[u8], cursor: usize) -> Option<u64> {
    Some(u64::from_be_bytes(bytes.get(cursor..cursor + 8)?.try_into().ok()?))
}

fn read_i32(bytes: &[u8], cursor: usize) -> Option<i32> {
    Some(i32::from_be_bytes(bytes.get(cursor..cursor + 4)?.try_into().ok()?))
}

fn movie_seconds(bytes: &[u8]) -> Option<u32> {
    let moov = top_level(bytes).into_iter().find(|item| item.kind == *b"moov")?;
    let mut timescale = 0u32;
    let mut duration = 0u64;
    let mut extended = 0u64;
    for child in children(bytes, &moov, 0) {
        if child.kind == *b"mvhd" {
            let (scale, span) = read_mvhd(bytes, &child)?;
            timescale = scale;
            duration = span;
        } else if child.kind == *b"mvex" {
            if let Some(span) = read_mehd(bytes, &child) {
                extended = span;
            }
        }
    }
    if timescale == 0 {
        return None;
    }
    let span = if duration > 0 { duration } else { extended };
    span_seconds(span, timescale)
}

fn read_mvhd(bytes: &[u8], item: &Mp4Box) -> Option<(u32, u64)> {
    let (version, _, mut cursor) = full_box(bytes, item).ok()?;
    if version == 1 {
        cursor = cursor.checked_add(16)?;
        let timescale = read_u32(bytes, cursor)?;
        let duration = read_u64(bytes, cursor.checked_add(4)?)?;
        Some((timescale, duration))
    } else {
        cursor = cursor.checked_add(8)?;
        let timescale = read_u32(bytes, cursor)?;
        let duration = u64::from(read_u32(bytes, cursor.checked_add(4)?)?);
        Some((timescale, duration))
    }
}

fn read_mehd(bytes: &[u8], mvex: &Mp4Box) -> Option<u64> {
    let mehd = children(bytes, mvex, 0).into_iter().find(|item| item.kind == *b"mehd")?;
    let (version, _, cursor) = full_box(bytes, &mehd).ok()?;
    if version == 1 { read_u64(bytes, cursor) } else { read_u32(bytes, cursor).map(u64::from) }
}

fn span_seconds(span: u64, timescale: u32) -> Option<u32> {
    if span == 0 || timescale == 0 {
        return None;
    }
    let seconds = (span + u64::from(timescale) / 2) / u64::from(timescale);
    u32::try_from(seconds).ok().filter(|seconds| *seconds > 0)
}

fn adts_seconds(bytes: &[u8]) -> Option<u32> {
    if bytes.len() < 7 || bytes[0] != 0xFF || bytes[1] & 0xF6 != 0xF0 {
        return None;
    }
    let rate = AAC_RATES.get(usize::from((bytes[2] >> 2) & 0x0F))?;
    if *rate == 0 {
        return None;
    }
    let mut pos = 0usize;
    let mut frames = 0u64;
    while pos + 7 <= bytes.len() {
        if bytes[pos] != 0xFF || bytes[pos + 1] & 0xF6 != 0xF0 {
            return None;
        }
        let frame_len = (usize::from(bytes[pos + 3] & 0x03) << 11) | (usize::from(bytes[pos + 4]) << 3) | (usize::from(bytes[pos + 5]) >> 5);
        if frame_len < 7 || pos + frame_len > bytes.len() {
            return None;
        }
        pos += frame_len;
        frames += 1;
    }
    if frames == 0 || pos != bytes.len() {
        return None;
    }
    span_seconds(frames * 1024, *rate)
}

const AAC_RATES: [u32; 16] = [96_000, 88_200, 64_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025, 8_000, 7_350, 0, 0, 0];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_track_is_removed_without_moving_the_audio() {
        let video = trak(b"vide");
        let sound = trak(b"soun");
        let moov = box_of(b"moov", &[box_of(b"mvhd", &[0; 24]), video, sound].concat());
        let prefix = [box_of(b"ftyp", b"mp42"), moov].concat();
        let mdat = box_of(b"mdat", b"AUDIO!!!");
        let mut file = prefix.clone();
        file.extend_from_slice(&mdat);
        let ready = playable(file.clone()).expect("playable");
        let original_mdat = file.windows(4).position(|chunk| chunk == b"mdat").unwrap();
        let ready_mdat = ready.windows(4).position(|chunk| chunk == b"mdat").unwrap();
        assert_eq!(ready_mdat, original_mdat);
        assert_eq!(&ready[ready_mdat + 4..ready_mdat + 12], b"AUDIO!!!");
        let head = &ready[..ready_mdat];
        assert!(head.windows(4).any(|chunk| chunk == b"soun"));
        assert!(!head.windows(4).any(|chunk| chunk == b"vide"));
    }

    fn trak(handler: &[u8; 4]) -> Vec<u8> {
        let mut hdlr = vec![0; 8];
        hdlr.extend_from_slice(handler);
        box_of(b"trak", &box_of(b"hdlr", &hdlr))
    }

    #[test]
    fn adts_header_for_aac_lc_stereo() {
        assert_eq!(adts_header(&[0x12, 0x10], 464), [0xFF, 0xF1, 0x50, 0x80, 0x3A, 0xFF, 0xFC]);
    }

    #[test]
    fn movie_header_is_the_song_length() {
        let mvhd = box_of(b"mvhd", &{
            let mut payload = Vec::new();
            payload.extend_from_slice(&0u32.to_be_bytes());
            payload.extend_from_slice(&0u32.to_be_bytes());
            payload.extend_from_slice(&0u32.to_be_bytes());
            payload.extend_from_slice(&1000u32.to_be_bytes());
            payload.extend_from_slice(&137_067u32.to_be_bytes());
            payload
        });
        let file = [box_of(b"ftyp", b"mp42"), box_of(b"moov", &mvhd)].concat();
        assert_eq!(length_seconds(&file), Some(137));
    }

    #[test]
    fn empty_movie_header_uses_the_fragment_length() {
        let mvhd = box_of(b"mvhd", &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x03, 0xE8, 0, 0, 0, 0]);
        let mehd = box_of(b"mehd", &{
            let mut payload = vec![0, 0, 0, 0];
            payload.extend_from_slice(&8_000u32.to_be_bytes());
            payload
        });
        let file = box_of(b"moov", &[mvhd, box_of(b"mvex", &mehd)].concat());
        assert_eq!(length_seconds(&file), Some(8));
    }

    #[test]
    fn adts_frames_add_up_to_seconds() {
        let header = adts_header(&[0x15, 0x88], 16);
        let mut file = Vec::new();
        for _ in 0..80 {
            file.extend_from_slice(&header);
            file.extend_from_slice(&[0; 16]);
        }
        assert_eq!(length_seconds(&file), Some(10));
    }

    fn box_of(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut out = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
        out.extend_from_slice(kind);
        out.extend_from_slice(payload);
        out
    }
}
