use tui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Spans, Text},
    widgets::{Block, StatefulWidget, Widget},
};

#[derive(Clone)]
pub struct LibState {
    offset: usize,
    selected: Option<usize>,
}

impl Default for LibState {
    fn default() -> LibState {
        LibState {
            offset: 0,
            selected: Some(0),
        }
    }
}

impl LibState {
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
        if index.is_none() {
            self.offset = 0;
        }
    }

    pub fn offset(&mut self, index: usize) {
        self.offset = index;
    }
}

#[derive(PartialEq)]
pub enum LibKind {
    Artist,
    Album,
    Title,
    Back,
    All,
    None,
}

impl Copy for LibKind {}

impl Clone for LibKind {
    fn clone(&self) -> Self {
        *self
    }
}

#[derive(Clone)]
pub struct LibItem {
    pub content: String,
    pub style: Style,
    pub tag: LibKind,
}

impl LibItem {
    pub fn new(content: String, tag: LibKind) -> LibItem {
        LibItem {
            content,
            style: Style::default(),
            tag,
        }
    }
}

pub struct Tree<'a> {
    block: Option<Block<'a>>,
    items: &'a Vec<LibItem>,
    style: Style,
    highlight_style: Style,
    highlight_symbol: Option<&'a str>,
}

impl<'a> Tree<'a> {
    pub fn new<T>(items: T) -> Tree<'a>
    where
        T: Into<&'a Vec<LibItem>>,
    {
        Tree {
            block: None,
            style: Style::default(),
            items: &items.into(),
            highlight_style: Style::default(),
            highlight_symbol: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Tree<'a> {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Tree<'a> {
        self.style = style;
        self
    }

    pub fn highlight_symbol(mut self, highlight_symbol: &'a str) -> Tree<'a> {
        self.highlight_symbol = Some(highlight_symbol);
        self
    }

    pub fn highlight_style(mut self, style: Style) -> Tree<'a> {
        self.highlight_style = style;
        self
    }

    fn get_items_bounds(
        &self,
        selected: Option<usize>,
        offset: usize,
        max_height: usize,
    ) -> (usize, usize) {
        let offset = offset.min(self.items.len().saturating_sub(1));
        let mut start = offset;
        let mut end = offset;
        let mut height = 0;
        for item in self.items.iter().skip(offset) {
            if height + Text::from(item.content.clone()).height() > max_height {
                break;
            }
            height += Text::from(item.content.clone()).height();
            end += 1;
        }

        let selected = selected.unwrap_or(0).min(self.items.len() - 1);
        while selected >= end {
            height = height.saturating_add(Text::from(self.items[end].content.clone()).height());
            end += 1;
            while height > max_height {
                height =
                    height.saturating_sub(Text::from(self.items[start].content.clone()).height());
                start += 1;
            }
        }
        while selected < start {
            start -= 1;
            height = height.saturating_add(Text::from(self.items[start].content.clone()).height());
            while height > max_height {
                end -= 1;
                height =
                    height.saturating_sub(Text::from(self.items[end].content.clone()).height());
            }
        }
        (start, end)
    }
}

impl<'a> StatefulWidget for Tree<'a> {
    type State = LibState;

    fn render(mut self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_style(area, self.style);
        let list_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };

        if list_area.width < 1 || list_area.height < 1 {
            return;
        }

        if self.items.is_empty() {
            return;
        }
        let list_height = list_area.height as usize;

        let (start, end) = self.get_items_bounds(state.selected, state.offset, list_height);
        state.offset = start;

        let highlight_symbol = self.highlight_symbol.unwrap_or("");

        let mut current_height = 0;
        let has_selection = state.selected.is_some();
        for (i, item) in self
            .items
            .iter()
            .enumerate()
            .skip(state.offset)
            .take(end - start)
        {
            let (x, y) = (list_area.left(), list_area.top() + current_height);
            current_height += Text::from(item.content.clone()).height() as u16;
            let area = Rect {
                x,
                y,
                width: list_area.width,
                height: Text::from(item.content.clone()).height() as u16,
            };

            let item_style = self.style.patch(item.style);
            buf.set_style(area, item_style);

            let is_selected = state.selected.map(|s| s == i).unwrap_or(false);

            let symbol = match item.tag {
                LibKind::Artist => String::from("  "),
                LibKind::Album => String::from("    "),
                LibKind::Title => String::from("     "),
                LibKind::None => String::from(" "),
                _ => String::from(""),
            };
            let (elem_x, max_element_width) = if has_selection {
                let (elem_x, _) =
                    buf.set_stringn(x, y, &symbol, list_area.width as usize, item_style);
                (elem_x, (list_area.width - (elem_x - x)) as u16)
            } else {
                (x, list_area.width)
            };
            buf.set_spans(
                elem_x,
                y,
                &Spans::from(item.content.clone()),
                max_element_width as u16,
            );
            let rect = Rect::new(
                symbol.chars().count() as u16,
                y,
                list_area.width - symbol.chars().count() as u16,
                1,
            );
            if is_selected {
                buf.set_style(rect, self.highlight_style);
            }
        }
    }
}
