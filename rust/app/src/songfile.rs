// A download should open in File Explorer and other players with its title, artist, and cover.

use crate::model::Track;

pub struct Stored {
    pub extension: &'static str,
    pub bytes: Vec<u8>,
}

pub fn store(bytes: &[u8], track: &Track, cover: Option<&[u8]>) -> Stored {
    let tags = Tags {
        title: &track.title,
        artist: &track.artist(),
        album: track.album.as_ref().map(|album| album.name.as_str()).unwrap_or(""),
        cover: picture(cover),
    };
    if let Some(audio) = adts_audio(bytes) {
        if let Some(stored) = aac_m4a(&audio, &tags) {
            return stored;
        }
        return Stored { extension: "aac", bytes: bytes.to_vec() };
    }
    if looks_like_mp4(bytes) {
        let tagged = tag_mp4(bytes, &tags).unwrap_or_else(|| bytes.to_vec());
        return Stored { extension: "m4a", bytes: tagged };
    }
    if is_mp3(bytes) {
        return Stored { extension: "mp3", bytes: tag_mp3(bytes, &tags) };
    }
    if bytes.starts_with(b"fLaC") {
        let tagged = tag_flac(bytes, &tags).unwrap_or_else(|| bytes.to_vec());
        return Stored { extension: "flac", bytes: tagged };
    }
    Stored { extension: plain_extension(bytes), bytes: bytes.to_vec() }
}

pub fn plain(bytes: &[u8]) -> Stored {
    Stored { extension: plain_extension(bytes), bytes: bytes.to_vec() }
}

fn plain_extension(bytes: &[u8]) -> &'static str {
    if adts_audio(bytes).is_some() {
        "aac"
    } else if looks_like_mp4(bytes) {
        "m4a"
    } else if is_mp3(bytes) {
        "mp3"
    } else if bytes.starts_with(b"fLaC") {
        "flac"
    } else if bytes.starts_with(b"OggS") {
        "ogg"
    } else if bytes.starts_with(b"RIFF") {
        "wav"
    } else {
        "audio"
    }
}

struct Tags<'a> {
    title: &'a str,
    artist: &'a str,
    album: &'a str,
    cover: Option<Cover<'a>>,
}

struct Cover<'a> {
    mime: &'static str,
    kind: u32,
    bytes: &'a [u8],
}

struct AdtsAudio<'a> {
    rate: u32,
    channels: u16,
    aot: u8,
    frames: Vec<&'a [u8]>,
}

fn picture(bytes: Option<&[u8]>) -> Option<Cover<'_>> {
    let bytes = bytes?;
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(Cover { mime: "image/jpeg", kind: 13, bytes })
    } else if bytes.starts_with(b"\x89PNG") {
        Some(Cover { mime: "image/png", kind: 14, bytes })
    } else {
        None
    }
}

fn clean(text: &str) -> String {
    text.chars().filter(|symbol| *symbol != '\0').take(300).collect()
}

fn adts_audio(bytes: &[u8]) -> Option<AdtsAudio<'_>> {
    if bytes.len() < 7 || bytes[0] != 0xFF || bytes[1] & 0xF6 != 0xF0 {
        return None;
    }
    let rate_index = usize::from((bytes[2] >> 2) & 0x0F);
    let rate = *AAC_RATES.get(rate_index)?;
    if rate == 0 {
        return None;
    }
    let channels = u16::from(((bytes[2] & 0x01) << 2) | ((bytes[3] >> 6) & 0x03));
    let aot = ((bytes[2] >> 6) & 0x03) + 1;
    if channels == 0 || aot > 4 {
        return None;
    }
    let mut pos = 0usize;
    let mut frames = Vec::new();
    while pos + 7 <= bytes.len() {
        if bytes[pos] != 0xFF || bytes[pos + 1] & 0xF6 != 0xF0 {
            return None;
        }
        let header = 7 + if bytes[pos + 1] & 0x01 == 0 { 2 } else { 0 };
        let frame_len = (usize::from(bytes[pos + 3] & 0x03) << 11) | (usize::from(bytes[pos + 4]) << 3) | (usize::from(bytes[pos + 5]) >> 5);
        if frame_len < header || pos + frame_len > bytes.len() {
            return None;
        }
        let same_rate = usize::from((bytes[pos + 2] >> 2) & 0x0F) == rate_index;
        let same_channels = u16::from(((bytes[pos + 2] & 0x01) << 2) | ((bytes[pos + 3] >> 6) & 0x03)) == channels;
        let same_aot = ((bytes[pos + 2] >> 6) & 0x03) + 1 == aot;
        if !same_rate || !same_channels || !same_aot {
            return None;
        }
        frames.push(&bytes[pos + header..pos + frame_len]);
        pos += frame_len;
    }
    if frames.is_empty() || pos != bytes.len() {
        return None;
    }
    Some(AdtsAudio { rate, channels, aot, frames })
}

fn aac_m4a(audio: &AdtsAudio<'_>, tags: &Tags<'_>) -> Option<Stored> {
    let asc = audio_specific_config(audio)?;
    let mut payload = Vec::new();
    let mut sizes = Vec::with_capacity(audio.frames.len());
    for frame in &audio.frames {
        sizes.push(u32::try_from(frame.len()).ok()?);
        payload.extend_from_slice(frame);
    }
    let samples = u32::try_from(audio.frames.len()).ok()?;
    let duration = u64::from(samples) * 1024;
    let ftyp = must_box(b"ftyp", b"M4A \x00\x00\x02\x00M4A isomiso2");
    let moov_len = movie(audio, &asc, &sizes, duration, 0, tags)?.len();
    let offset = u32::try_from(ftyp.len() + moov_len + 8).ok()?;
    let moov = movie(audio, &asc, &sizes, duration, offset, tags)?;
    let mdat = must_box(b"mdat", &payload);
    let mut bytes = ftyp;
    bytes.extend(moov);
    bytes.extend(mdat);
    Some(Stored { extension: "m4a", bytes })
}

fn audio_specific_config(audio: &AdtsAudio<'_>) -> Option<[u8; 2]> {
    let index = AAC_RATES.iter().position(|rate| *rate == audio.rate)?;
    if index > 0x0E || audio.channels > 7 {
        return None;
    }
    let aot = audio.aot;
    let freq = index as u8;
    let channels = audio.channels as u8;
    Some([(aot << 3) | (freq >> 1), ((freq & 1) << 7) | (channels << 3)])
}

fn movie(audio: &AdtsAudio<'_>, asc: &[u8], sizes: &[u32], duration: u64, offset: u32, tags: &Tags<'_>) -> Option<Vec<u8>> {
    let timescale = audio.rate;
    let span = u32::try_from(duration).ok()?;
    let samples = sizes.len() as u32;
    let bitrate = u32::try_from((u64::from(sizes.iter().sum::<u32>()) * 8 * u64::from(timescale)) / duration.max(1)).unwrap_or(128_000);
    let body = media(audio, asc, sizes, timescale, span, samples, bitrate, offset)?;
    let trak = must_box(b"trak", &[track_header(span), edit_list(span), body].concat());
    Some(must_box(b"moov", &[movie_header(timescale, span), trak, user_data(tags)].concat()))
}

fn movie_header(timescale: u32, duration: u32) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&timescale.to_be_bytes());
    body.extend_from_slice(&duration.to_be_bytes());
    body.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    body.extend_from_slice(&0x0100u16.to_be_bytes());
    body.extend_from_slice(&[0, 0]);
    body.extend_from_slice(&[0; 8]);
    body.extend_from_slice(&MATRIX);
    body.extend_from_slice(&[0; 24]);
    body.extend_from_slice(&2u32.to_be_bytes());
    full_box(b"mvhd", &body)
}

fn track_header(duration: u32) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&1u32.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&duration.to_be_bytes());
    body.extend_from_slice(&[0; 8]);
    body.extend_from_slice(&0u16.to_be_bytes());
    body.extend_from_slice(&0u16.to_be_bytes());
    body.extend_from_slice(&0x0100u16.to_be_bytes());
    body.extend_from_slice(&0u16.to_be_bytes());
    body.extend_from_slice(&MATRIX);
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    let mut payload = 0x0000_0003u32.to_be_bytes().to_vec();
    payload.extend(body);
    must_box(b"tkhd", &payload)
}

fn edit_list(duration: u32) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&1u32.to_be_bytes());
    body.extend_from_slice(&duration.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    must_box(b"edts", &full_box(b"elst", &body))
}

fn media(audio: &AdtsAudio<'_>, asc: &[u8], sizes: &[u32], timescale: u32, duration: u32, samples: u32, bitrate: u32, offset: u32) -> Option<Vec<u8>> {
    let mdhd = media_header(timescale, duration);
    let hdlr = sound_handler();
    let table = sample_table(audio, asc, sizes, samples, bitrate, offset)?;
    let minf = must_box(b"minf", &[full_box(b"smhd", &[0, 0, 0, 0]), data_reference(), table].concat());
    Some(must_box(b"mdia", &[mdhd, hdlr, minf].concat()))
}

fn media_header(timescale: u32, duration: u32) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(&timescale.to_be_bytes());
    body.extend_from_slice(&duration.to_be_bytes());
    body.extend_from_slice(&0x55C4u16.to_be_bytes());
    body.extend_from_slice(&0u16.to_be_bytes());
    full_box(b"mdhd", &body)
}

fn sound_handler() -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(b"soun");
    body.extend_from_slice(&[0; 12]);
    body.extend_from_slice(b"SoundHandler\0");
    full_box(b"hdlr", &body)
}

fn data_reference() -> Vec<u8> {
    let url = full_box(b"url ", &[]);
    let mut flags = url;
    // Self-contained flag lives in the version/flags word. full_box wrote zeros, so set the flag.
    let flag_at = flags.len() - 4;
    flags[flag_at + 3] = 1;
    let mut body = 1u32.to_be_bytes().to_vec();
    body.extend(flags);
    must_box(b"dinf", &full_box(b"dref", &body))
}

fn sample_table(audio: &AdtsAudio<'_>, asc: &[u8], sizes: &[u32], samples: u32, bitrate: u32, offset: u32) -> Option<Vec<u8>> {
    let stsd = full_box(b"stsd", &[1u32.to_be_bytes().as_slice(), &sample_entry(audio, asc, bitrate)?].concat());
    let stts = full_box(b"stts", &[1u32.to_be_bytes().as_slice(), &samples.to_be_bytes(), &1024u32.to_be_bytes()].concat());
    let stsc = full_box(b"stsc", &[1u32.to_be_bytes().as_slice(), &1u32.to_be_bytes(), &samples.to_be_bytes(), &1u32.to_be_bytes()].concat());
    let mut stsz_body = 0u32.to_be_bytes().to_vec();
    stsz_body.extend_from_slice(&samples.to_be_bytes());
    for size in sizes {
        stsz_body.extend_from_slice(&size.to_be_bytes());
    }
    let stsz = full_box(b"stsz", &stsz_body);
    let stco = full_box(b"stco", &[1u32.to_be_bytes().as_slice(), &offset.to_be_bytes()].concat());
    Some(must_box(b"stbl", &[stsd, stts, stsc, stsz, stco].concat()))
}

fn sample_entry(audio: &AdtsAudio<'_>, asc: &[u8], bitrate: u32) -> Option<Vec<u8>> {
    let mut entry = vec![0; 28];
    entry[6] = 0;
    entry[7] = 1;
    entry[16..18].copy_from_slice(&audio.channels.to_be_bytes());
    entry[18..20].copy_from_slice(&16u16.to_be_bytes());
    entry[24..26].copy_from_slice(&(audio.rate as u16).to_be_bytes());
    entry.extend(descriptor_box(asc, bitrate)?);
    box_of(b"mp4a", &entry)
}

fn descriptor_box(asc: &[u8], bitrate: u32) -> Option<Vec<u8>> {
    let specific = descriptor(0x05, asc);
    let mut decoder = Vec::new();
    decoder.push(0x40);
    decoder.push(0x15);
    decoder.extend_from_slice(&[0, 0, 0]);
    decoder.extend_from_slice(&bitrate.to_be_bytes());
    decoder.extend_from_slice(&bitrate.to_be_bytes());
    decoder.extend(specific);
    let mut elementary = Vec::new();
    elementary.extend_from_slice(&1u16.to_be_bytes());
    elementary.push(0);
    elementary.extend(descriptor(0x04, &decoder));
    elementary.extend(descriptor(0x06, &[0x02]));
    let mut payload = vec![0, 0, 0, 0];
    payload.extend(descriptor(0x03, &elementary));
    box_of(b"esds", &payload)
}

fn descriptor(tag: u8, body: &[u8]) -> Vec<u8> {
    let len = body.len();
    let mut out = vec![
        tag,
        0x80 | ((len >> 21) as u8 & 0x7F),
        0x80 | ((len >> 14) as u8 & 0x7F),
        0x80 | ((len >> 7) as u8 & 0x7F),
        (len & 0x7F) as u8,
    ];
    out.extend_from_slice(body);
    out
}

fn tag_mp4(bytes: &[u8], tags: &Tags<'_>) -> Option<Vec<u8>> {
    let boxes = boxes_in(bytes, 0, bytes.len());
    let moov = boxes.iter().find(|item| item.kind == *b"moov")?;
    let mut payload = Vec::new();
    for child in boxes_in(bytes, moov.start + moov.header, moov.start + moov.size) {
        if child.kind == *b"udta" {
            continue;
        }
        payload.extend_from_slice(&bytes[child.start..child.start + child.size]);
    }
    payload.extend(user_data(tags));
    let mut moov_bytes = box_of(b"moov", &payload)?;
    let old_end = moov.start + moov.size;
    let mdat_after = boxes.iter().any(|item| item.kind == *b"mdat" && item.start >= old_end);
    if mdat_after {
        let delta = moov_bytes.len() as i64 - moov.size as i64;
        if !shift_chunks(&mut moov_bytes, old_end as u64, delta) {
            return None;
        }
    }
    let mut out = Vec::with_capacity(bytes.len() + moov_bytes.len());
    out.extend_from_slice(&bytes[..moov.start]);
    out.extend_from_slice(&moov_bytes);
    out.extend_from_slice(&bytes[old_end..]);
    Some(out)
}

fn shift_chunks(bytes: &mut [u8], threshold: u64, delta: i64) -> bool {
    if delta == 0 {
        return true;
    }
    shift_range(bytes, 0, bytes.len(), threshold, delta)
}

fn shift_range(bytes: &mut [u8], start: usize, end: usize, threshold: u64, delta: i64) -> bool {
    let mut pos = start;
    while pos + 8 <= end {
        let Some(item) = read_box(bytes, pos, end) else { return false };
        let next = item.start + item.size;
        let ok = match &item.kind {
            b"stco" => shift_entries(bytes, &item, false, threshold, delta),
            b"co64" => shift_entries(bytes, &item, true, threshold, delta),
            b"moov" | b"trak" | b"mdia" | b"minf" | b"stbl" | b"edts" | b"mvex" | b"moof" | b"traf" | b"dinf" => {
                shift_range(bytes, item.start + item.header, next, threshold, delta)
            }
            _ => true,
        };
        if !ok {
            return false;
        }
        pos = next;
    }
    true
}

fn shift_entries(bytes: &mut [u8], item: &Mp4Box, wide: bool, threshold: u64, delta: i64) -> bool {
    let cursor = item.start + item.header + 4;
    if cursor + 4 > item.start + item.size {
        return false;
    }
    let count = u32::from_be_bytes(bytes[cursor..cursor + 4].try_into().unwrap_or([0; 4])) as usize;
    let width = if wide { 8 } else { 4 };
    let table = cursor + 4;
    let end = table.saturating_add(count.saturating_mul(width));
    if end > item.start + item.size {
        return false;
    }
    for index in 0..count {
        let at = table + index * width;
        let current = if wide {
            u64::from_be_bytes(bytes[at..at + 8].try_into().unwrap_or([0; 8]))
        } else {
            u64::from(u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap_or([0; 4])))
        };
        let updated = moved_offset(current, threshold, delta);
        if !wide && updated > u64::from(u32::MAX) {
            return false;
        }
        if wide {
            bytes[at..at + 8].copy_from_slice(&updated.to_be_bytes());
        } else {
            bytes[at..at + 4].copy_from_slice(&(updated as u32).to_be_bytes());
        }
    }
    true
}

fn moved_offset(offset: u64, threshold: u64, delta: i64) -> u64 {
    if offset < threshold || delta == 0 {
        return offset;
    }
    if delta > 0 { offset.saturating_add(delta as u64) } else { offset.saturating_sub(delta.unsigned_abs()) }
}

fn user_data(tags: &Tags<'_>) -> Vec<u8> {
    let ilst = item_list(tags);
    let hdlr = metadata_handler();
    let mut meta = vec![0, 0, 0, 0];
    meta.extend(hdlr);
    meta.extend(ilst);
    box_of(b"udta", &box_of(b"meta", &meta).unwrap_or_default()).unwrap_or_default()
}

fn metadata_handler() -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&0u32.to_be_bytes());
    body.extend_from_slice(b"mdir");
    body.extend_from_slice(b"appl");
    body.extend_from_slice(&[0; 8]);
    body.push(0);
    full_box(b"hdlr", &body)
}

fn item_list(tags: &Tags<'_>) -> Vec<u8> {
    let mut items = Vec::new();
    if let Some(value) = text_atom([0xA9, b'n', b'a', b'm'], &clean(tags.title)) {
        items.extend(value);
    }
    if let Some(value) = text_atom([0xA9, b'A', b'R', b'T'], &clean(tags.artist)) {
        items.extend(value);
    }
    if let Some(value) = text_atom([0xA9, b'a', b'l', b'b'], &clean(tags.album)) {
        items.extend(value);
    }
    if let Some(cover) = &tags.cover {
        if let Some(value) = data_atom(*b"covr", cover.kind, cover.bytes) {
            items.extend(value);
        }
    }
    box_of(b"ilst", &items).unwrap_or_default()
}

fn text_atom(kind: [u8; 4], text: &str) -> Option<Vec<u8>> {
    if text.is_empty() {
        return None;
    }
    data_atom(kind, 1, text.as_bytes())
}

fn data_atom(kind: [u8; 4], class: u32, value: &[u8]) -> Option<Vec<u8>> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&class.to_be_bytes());
    payload.extend_from_slice(&0u32.to_be_bytes());
    payload.extend_from_slice(value);
    box_of(&kind, &box_of(b"data", &payload)?)
}

fn is_mp3(bytes: &[u8]) -> bool {
    let audio = after_id3(bytes);
    audio.len() >= 2 && audio[0] == 0xFF && audio[1] & 0xE0 == 0xE0 && ((audio[1] >> 1) & 0x03) != 0
}

fn after_id3(bytes: &[u8]) -> &[u8] {
    if bytes.len() < 10 || !bytes.starts_with(b"ID3") {
        return bytes;
    }
    let size = synchsafe(&bytes[6..10]);
    let mut total = 10 + size;
    if bytes[5] & 0x10 != 0 {
        total += 10;
    }
    if total < bytes.len() { &bytes[total..] } else { bytes }
}

fn tag_mp3(bytes: &[u8], tags: &Tags<'_>) -> Vec<u8> {
    let audio = after_id3(bytes);
    let mut frames = Vec::new();
    if let Some(frame) = text_frame(b"TIT2", &clean(tags.title)) {
        frames.extend(frame);
    }
    if let Some(frame) = text_frame(b"TPE1", &clean(tags.artist)) {
        frames.extend(frame);
    }
    if let Some(frame) = text_frame(b"TALB", &clean(tags.album)) {
        frames.extend(frame);
    }
    if let Some(cover) = &tags.cover {
        frames.extend(picture_frame(cover));
    }
    let mut out = vec![b'I', b'D', b'3', 3, 0, 0];
    out.extend_from_slice(&write_synchsafe(frames.len() as u32));
    out.extend(frames);
    out.extend_from_slice(audio);
    out
}

fn text_frame(id: &[u8; 4], text: &str) -> Option<Vec<u8>> {
    if text.is_empty() {
        return None;
    }
    let mut body = vec![1, 0xFF, 0xFE];
    for unit in text.encode_utf16() {
        body.extend_from_slice(&unit.to_le_bytes());
    }
    Some(id3_frame(id, &body))
}

fn picture_frame(cover: &Cover<'_>) -> Vec<u8> {
    let mut body = vec![0];
    body.extend_from_slice(cover.mime.as_bytes());
    body.push(0);
    body.push(3);
    body.push(0);
    body.extend_from_slice(cover.bytes);
    id3_frame(b"APIC", &body)
}

fn id3_frame(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut frame = id.to_vec();
    frame.extend_from_slice(&(body.len() as u32).to_be_bytes());
    frame.extend_from_slice(&[0, 0]);
    frame.extend_from_slice(body);
    frame
}

fn synchsafe(bytes: &[u8]) -> usize {
    bytes.iter().take(4).fold(0usize, |size, byte| (size << 7) | usize::from(byte & 0x7F))
}

fn write_synchsafe(size: u32) -> [u8; 4] {
    [(size >> 21) as u8 & 0x7F, (size >> 14) as u8 & 0x7F, (size >> 7) as u8 & 0x7F, size as u8 & 0x7F]
}

fn tag_flac(bytes: &[u8], tags: &Tags<'_>) -> Option<Vec<u8>> {
    if !bytes.starts_with(b"fLaC") {
        return None;
    }
    let mut pos = 4usize;
    let mut blocks = Vec::new();
    loop {
        if pos + 4 > bytes.len() {
            return None;
        }
        let last = bytes[pos] & 0x80 != 0;
        let kind = bytes[pos] & 0x7F;
        let len = (usize::from(bytes[pos + 1]) << 16) | (usize::from(bytes[pos + 2]) << 8) | usize::from(bytes[pos + 3]);
        let start = pos + 4;
        let end = start.checked_add(len)?;
        if end > bytes.len() {
            return None;
        }
        if kind != 4 && kind != 6 {
            blocks.push((kind, bytes[start..end].to_vec()));
        }
        pos = end;
        if last {
            break;
        }
    }
    if blocks.first().map(|(kind, _)| *kind != 0).unwrap_or(true) {
        return None;
    }
    blocks.push((4, vorbis_comment(tags)));
    if let Some(cover) = &tags.cover {
        blocks.push((6, flac_picture(cover)?));
    }
    let mut out = b"fLaC".to_vec();
    for (index, (kind, data)) in blocks.iter().enumerate() {
        if data.len() > 0x00FF_FFFF {
            return None;
        }
        let last = if index + 1 == blocks.len() { 0x80 } else { 0 };
        out.push(last | kind);
        out.push((data.len() >> 16) as u8);
        out.push((data.len() >> 8) as u8);
        out.push(data.len() as u8);
        out.extend_from_slice(data);
    }
    out.extend_from_slice(&bytes[pos..]);
    Some(out)
}

fn vorbis_comment(tags: &Tags<'_>) -> Vec<u8> {
    let mut comments = Vec::new();
    push_comment(&mut comments, "TITLE", &clean(tags.title));
    push_comment(&mut comments, "ARTIST", &clean(tags.artist));
    push_comment(&mut comments, "ALBUM", &clean(tags.album));
    let vendor = b"ValDex";
    let mut out = Vec::new();
    out.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    out.extend_from_slice(vendor);
    out.extend_from_slice(&(comments.len() as u32).to_le_bytes());
    for comment in comments {
        out.extend_from_slice(&(comment.len() as u32).to_le_bytes());
        out.extend(comment);
    }
    out
}

fn push_comment(comments: &mut Vec<Vec<u8>>, key: &str, value: &str) {
    if value.is_empty() {
        return;
    }
    comments.push(format!("{key}={value}").into_bytes());
}

fn flac_picture(cover: &Cover<'_>) -> Option<Vec<u8>> {
    let (width, height) = image_size(cover.bytes).unwrap_or((0, 0));
    let mut out = Vec::new();
    out.extend_from_slice(&3u32.to_be_bytes());
    out.extend_from_slice(&(cover.mime.len() as u32).to_be_bytes());
    out.extend_from_slice(cover.mime.as_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&width.to_be_bytes());
    out.extend_from_slice(&height.to_be_bytes());
    out.extend_from_slice(&24u32.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&(cover.bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(cover.bytes);
    Some(out)
}

fn image_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.starts_with(b"\x89PNG") && bytes.len() >= 24 {
        let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
        let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
        return Some((width, height));
    }
    let mut pos = 2usize;
    while pos + 8 < bytes.len() && bytes[pos] == 0xFF {
        let marker = bytes[pos + 1];
        let len = u16::from_be_bytes(bytes.get(pos + 2..pos + 4)?.try_into().ok()?) as usize;
        if marker == 0xC0 || marker == 0xC2 {
            let height = u16::from_be_bytes(bytes.get(pos + 5..pos + 7)?.try_into().ok()?) as u32;
            let width = u16::from_be_bytes(bytes.get(pos + 7..pos + 9)?.try_into().ok()?) as u32;
            return Some((width, height));
        }
        pos = pos.checked_add(2 + len)?;
    }
    None
}

fn looks_like_mp4(bytes: &[u8]) -> bool {
    let boxes = boxes_in(bytes, 0, bytes.len());
    boxes.iter().any(|item| matches!(&item.kind, b"ftyp" | b"moov" | b"mdat" | b"free")) && boxes.iter().any(|item| item.kind == *b"moov")
}

struct Mp4Box {
    start: usize,
    size: usize,
    header: usize,
    kind: [u8; 4],
}

fn boxes_in(bytes: &[u8], start: usize, end: usize) -> Vec<Mp4Box> {
    let mut items = Vec::new();
    let mut pos = start;
    while pos + 8 <= end {
        let Some(item) = read_box(bytes, pos, end) else { break };
        pos = item.start + item.size;
        items.push(item);
    }
    items
}

fn read_box(bytes: &[u8], pos: usize, end: usize) -> Option<Mp4Box> {
    if pos + 8 > end || pos + 8 > bytes.len() {
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
    } else if size == 0 {
        size = end.saturating_sub(pos);
    }
    if size < header || pos + size > end || pos + size > bytes.len() {
        return None;
    }
    Some(Mp4Box { start: pos, size, header, kind })
}

fn must_box(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    box_of(kind, payload).unwrap_or_default()
}

fn full_box(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut payload = vec![0, 0, 0, 0];
    payload.extend_from_slice(body);
    box_of(kind, &payload).unwrap_or_default()
}

fn box_of(kind: &[u8; 4], payload: &[u8]) -> Option<Vec<u8>> {
    let size = u32::try_from(payload.len().checked_add(8)?).ok()?;
    let mut out = Vec::with_capacity(size as usize);
    out.extend_from_slice(&size.to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(payload);
    Some(out)
}

const MATRIX: [u8; 36] = [
    0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x40, 0x00, 0x00, 0x00,
];

const AAC_RATES: [u32; 16] = [96_000, 88_200, 64_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025, 8_000, 7_350, 0, 0, 0];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Album, Source as Catalog, Track};

    fn sample(title: &str) -> Track {
        Track {
            id: "song".into(),
            source: Some(Catalog::Youtube),
            title: title.into(),
            artists: vec![crate::model::Person { name: "Night Owls".into(), id: None }],
            album: Some(Album { name: "After Dark".into(), id: None }),
            duration: None,
            thumbnail: String::new(),
        }
    }

    fn opens(bytes: &[u8]) -> bool {
        rodio::Decoder::try_from(std::io::Cursor::new(bytes.to_vec())).is_ok_and(|decoder| decoder.take(32).count() >= 8)
    }

    #[test]
    fn a_cover_does_not_move_the_sound_when_the_movie_is_first() {
        let file = positioned(true);
        let stored = store(&file, &sample("Night Drive"), Some(include_bytes!("../fixtures/cover.jpg")));
        assert_eq!(stored.extension, "m4a");
        assert!(stored.bytes.windows(11).any(|chunk| chunk == b"Night Drive"));
        assert!(stored.bytes.windows(3).any(|chunk| chunk == [0xFF, 0xD8, 0xFF]));
        assert_eq!(payload_at_chunk(&stored.bytes), b"AUDIO!!!");
    }

    #[test]
    fn a_cover_keeps_the_sound_put_when_the_movie_is_last() {
        let file = positioned(false);
        let stored = store(&file, &sample("Night Drive"), Some(include_bytes!("../fixtures/cover.jpg")));
        assert_eq!(payload_at_chunk(&stored.bytes), b"AUDIO!!!");
    }

    #[test]
    fn adts_becomes_a_music_file_the_player_can_open() {
        let stored = store(include_bytes!("../fixtures/silence.aac"), &sample("Quiet"), Some(include_bytes!("../fixtures/cover.jpg")));
        assert_eq!(stored.extension, "m4a");
        assert!(stored.bytes.windows(5).any(|chunk| chunk == b"Quiet"));
        assert!(opens(&stored.bytes), "wrapped aac did not decode");
    }

    #[test]
    fn an_mp4_keeps_playing_with_the_title_inside() {
        let stored = store(include_bytes!("../fixtures/silence.m4a"), &sample("Quiet"), Some(include_bytes!("../fixtures/cover.jpg")));
        assert_eq!(stored.extension, "m4a");
        assert!(stored.bytes.windows(5).any(|chunk| chunk == b"Quiet"));
        assert!(opens(&stored.bytes), "tagged m4a did not decode");
    }

    #[test]
    fn an_mp3_keeps_playing_with_the_title_inside() {
        let stored = store(include_bytes!("../fixtures/silence.mp3"), &sample("Quiet"), Some(include_bytes!("../fixtures/cover.jpg")));
        assert_eq!(stored.extension, "mp3");
        assert!(stored.bytes.starts_with(b"ID3"));
        assert!(stored.bytes.windows(b"Night Owls".len()).any(|chunk| chunk == b"Night Owls") || utf16_contains(&stored.bytes, "Night Owls"));
        assert!(opens(&stored.bytes), "tagged mp3 did not decode");
    }

    #[test]
    fn a_flac_keeps_playing_with_the_title_inside() {
        let stored = store(include_bytes!("../fixtures/silence.flac"), &sample("Quiet"), Some(include_bytes!("../fixtures/cover.jpg")));
        assert_eq!(stored.extension, "flac");
        assert!(stored.bytes.windows(11).any(|chunk| chunk == b"TITLE=Quiet"));
        assert!(opens(&stored.bytes), "tagged flac did not decode");
    }

    fn utf16_contains(bytes: &[u8], text: &str) -> bool {
        let mut encoded = vec![0xFF, 0xFE];
        for unit in text.encode_utf16() {
            encoded.extend_from_slice(&unit.to_le_bytes());
        }
        bytes.windows(encoded.len()).any(|chunk| chunk == encoded)
    }

    fn payload_at_chunk(bytes: &[u8]) -> &[u8] {
        let at = bytes.windows(4).position(|chunk| chunk == b"stco").expect("stco");
        let offset = u32::from_be_bytes(bytes[at + 12..at + 16].try_into().unwrap()) as usize;
        &bytes[offset..offset + 8]
    }

    fn positioned(movie_first: bool) -> Vec<u8> {
        let mdat = box_of(b"mdat", b"AUDIO!!!").unwrap();
        let moov_shell = |offset: u32| {
            let stco = full_box(b"stco", &[1u32.to_be_bytes().as_slice(), &offset.to_be_bytes()].concat());
            let stbl = box_of(b"stbl", &stco).unwrap();
            let minf = box_of(b"minf", &stbl).unwrap();
            let mdia = box_of(b"mdia", &minf).unwrap();
            let trak = box_of(b"trak", &mdia).unwrap();
            box_of(b"moov", &trak).unwrap()
        };
        if movie_first {
            let moov = moov_shell(0);
            let offset = (moov.len() + 8) as u32;
            let moov = moov_shell(offset);
            [moov, mdat].concat()
        } else {
            let offset = 8u32;
            let moov = moov_shell(offset);
            [mdat, moov].concat()
        }
    }
}
