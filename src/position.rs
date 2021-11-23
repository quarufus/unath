use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Widget},
};

pub struct PositionWidget<'a> {
    block: Option<Block<'a>>,
    ratio: f64,
    style: Style,
}

impl<'a> Default for PositionWidget<'a> {
    fn default() -> PositionWidget<'a> {
        PositionWidget {
            block: None,
            ratio: 0.0,
            style: Style::default(),
        }
    }
}

impl<'a> PositionWidget<'a> {
    pub fn block(mut self, block: Block<'a>) -> PositionWidget<'a> {
        self.block = Some(block);
        self
    }

    pub fn ratio(mut self, ratio: f64) -> PositionWidget<'a> {
        assert!(
            (0.0..=1.0).contains(&ratio),
            "Ratio should be between 0 and 1 inclusively."
        );
        self.ratio = ratio;
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl<'a> Widget for PositionWidget<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        let position_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };
        buf.set_style(position_area, self.style);
        if position_area.height < 1 {
            return;
        }

        let filled_width = f64::from(position_area.width) * self.ratio;
        let end = position_area.left() + filled_width.floor() as u16;
        for y in position_area.top()..position_area.bottom() {
            for x in position_area.left()..end {
                buf.get_mut(x, y).set_symbol("─"); //;*/ ("━");
            }
            if self.ratio < 1.0 {
                buf.get_mut(end, y)
                    .set_symbol(get_unicode(filled_width % 1.0));

                for x in (end + 1)..position_area.right() {
                    buf.get_mut(x, y)
                        .set_symbol(" ")
                        .set_fg(self.style.fg.unwrap_or(Color::White))
                        .set_bg(self.style.bg.unwrap_or(Color::Black));
                }
            }
        }
    }
}

fn get_unicode<'a>(frac: f64) -> &'a str {
    match (frac * 2.0).floor() as u16 {
        0 => "╸", //"╾",
        1 => "╼", //"━",
        _ => " ",
    }
}
