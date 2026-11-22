//! Squish a control on press, then pop it back to its normal size.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::time::Instant;
use iced::{Element, Event, Length, Rectangle, Size, Transformation, mouse, touch, window};

const REST: f32 = 1.0;
const SQUISH: f32 = 0.92;
const POP: f32 = 1.045;
const DOWN_MS: f32 = 90.0;
const BACK_MS: f32 = 190.0;

pub fn press<'a, Message: 'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    Press { content: content.into() }.into()
}

struct Press<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
}

#[derive(Clone, Copy)]
enum Phase {
    Rest,
    Down { from: f32, started: Instant },
    Back { from: f32, started: Instant },
}

struct Tap {
    phase: Phase,
    held: bool,
    now: Instant,
}

impl Tap {
    fn rest() -> Self {
        Self { phase: Phase::Rest, held: false, now: Instant::now() }
    }

    fn sample(&self, now: Instant) -> f32 {
        match self.phase {
            Phase::Rest => REST,
            Phase::Down { from, started } => {
                let t = elapsed_ms(now, started) / DOWN_MS;
                if t >= 1.0 { SQUISH } else { from + (SQUISH - from) * ease_out_cubic(t) }
            }
            Phase::Back { from, started } => {
                let t = elapsed_ms(now, started) / BACK_MS;
                if t >= 1.0 { REST } else { settle(t, from) }
            }
        }
    }

    fn alive(&self, now: Instant) -> bool {
        match self.phase {
            Phase::Rest => false,
            Phase::Down { started, .. } => elapsed_ms(now, started) < DOWN_MS,
            Phase::Back { started, .. } => elapsed_ms(now, started) < BACK_MS,
        }
    }

    fn press(&mut self, now: Instant) {
        let from = self.sample(now);
        self.phase = Phase::Down { from, started: now };
        self.held = true;
        self.now = now;
    }

    fn release(&mut self, now: Instant) {
        if !self.held {
            return;
        }
        let seen = self.sample(now);
        self.phase = Phase::Back { from: seen.min(0.94), started: now };
        self.held = false;
        self.now = now;
    }
}

fn elapsed_ms(now: Instant, started: Instant) -> f32 {
    now.checked_duration_since(started).map(|span| span.as_secs_f32() * 1000.0).unwrap_or(0.0)
}

fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

fn settle(t: f32, from: f32) -> f32 {
    if t < 0.42 {
        let u = ease_out_cubic(t / 0.42);
        from + (POP - from) * u
    } else {
        let u = ease_out_cubic((t - 0.42) / 0.58);
        POP + (REST - POP) * u
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Press<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::renderer::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Tap>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Tap::rest())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        {
            let tap = tree.state.downcast_mut::<Tap>();
            let over = cursor.is_over(layout.bounds());
            let now = Instant::now();
            match event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if over => {
                    tap.press(now);
                    shell.request_redraw();
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left) | mouse::Event::CursorLeft) if tap.held => {
                    tap.release(now);
                    shell.request_redraw();
                }
                Event::Touch(touch::Event::FingerPressed { .. }) if over => {
                    tap.press(now);
                    shell.request_redraw();
                }
                Event::Touch(touch::Event::FingerLifted { .. } | touch::Event::FingerLost { .. }) if tap.held => {
                    tap.release(now);
                    shell.request_redraw();
                }
                Event::Window(window::Event::RedrawRequested(at)) => {
                    tap.now = *at;
                    if tap.alive(*at) {
                        shell.request_redraw();
                    }
                }
                _ => {}
            }
        }

        self.content.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let scale = {
            let tap = tree.state.downcast_ref::<Tap>();
            tap.sample(tap.now)
        };
        let draw_child = |renderer: &mut Renderer| {
            self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
        };
        if (scale - REST).abs() < 0.004 {
            draw_child(renderer);
            return;
        }
        let bounds = layout.bounds();
        let transform = Transformation::translate(bounds.x + bounds.width / 2.0, bounds.y + bounds.height / 2.0)
            * Transformation::scale(scale)
            * Transformation::translate(-bounds.x - bounds.width / 2.0, -bounds.y - bounds.height / 2.0);
        renderer.with_transformation(transform, draw_child);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

impl<'a, Message, Theme, Renderer> From<Press<'a, Message, Theme, Renderer>> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::renderer::Renderer + 'a,
{
    fn from(press: Press<'a, Message, Theme, Renderer>) -> Self {
        Element::new(press)
    }
}

#[cfg(test)]
mod tests {
    use super::{BACK_MS, SQUISH, Tap};
    use iced::time::Instant;
    use std::time::Duration;

    #[test]
    fn a_click_squishes_then_pops_back_to_rest() {
        let mut tap = Tap::rest();
        let start = Instant::now();
        tap.press(start);
        let held = tap.sample(start + Duration::from_millis(120));
        assert!((held - SQUISH).abs() < 0.001, "{held}");
        tap.release(start + Duration::from_millis(120));
        let mut peak = 0.0_f32;
        for step in 0..=200 {
            peak = peak.max(tap.sample(start + Duration::from_millis(120 + step)));
        }
        assert!(peak > 1.02, "{peak}");
        let done = tap.sample(start + Duration::from_millis(120 + BACK_MS as u64 + 30));
        assert!((done - 1.0).abs() < 0.001, "{done}");
    }

    #[test]
    fn a_fast_click_still_dips() {
        let mut tap = Tap::rest();
        let start = Instant::now();
        tap.press(start);
        tap.release(start);
        let dipped = tap.sample(start);
        assert!(dipped < 0.95, "{dipped}");
        let done = tap.sample(start + Duration::from_millis(400));
        assert!((done - 1.0).abs() < 0.001, "{done}");
    }
}
