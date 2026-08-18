//! Typed rendering for a complete `ui-choice` fenced response.

use ratatui::style::{Color, Modifier, Style};

use crate::scrollback::block::BlockContent;
use crate::scrollback::types::{AccentStyle, BlockContext, BlockOutput};
use xai_grok_markdown::MarkdownRenderView;

/// A parsed, render-only choice request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceBlock {
    pub prompt: Option<String>,
    pub options: Vec<String>,
    selected: usize,
    submitted: bool,
}

impl ChoiceBlock {
    /// Extract a valid choice from the renderer's recorded fenced-code spans.
    ///
    /// The info string match is deliberately exact. A fence with no options is
    /// malformed and is left to the normal markdown code-block renderer.
    pub fn from_view(view: &MarkdownRenderView<'_>) -> Option<Self> {
        view.code_blocks
            .iter()
            .find(|span| span.info == "ui-choice")
            .and_then(|span| Self::parse(&span.body))
    }

    fn parse(body: &str) -> Option<Self> {
        let mut lines = body.lines();
        let first = lines.next();
        let prompt = first
            .and_then(|line| line.strip_prefix("prompt: "))
            .map(str::to_owned);
        let options = first
            .into_iter()
            .filter(|_| prompt.is_none())
            .chain(lines)
            .filter_map(|line| line.strip_prefix("- "))
            .map(str::to_owned)
            .collect::<Vec<_>>();

        (!options.is_empty()).then_some(Self {
            prompt,
            options,
            selected: 0,
            submitted: false,
        })
    }

    /// Move the highlighted option, wrapping at either end.
    pub fn move_selection(&mut self, delta: isize) {
        let len = self.options.len() as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(len) as usize;
    }

    /// Text to send when the highlighted option is accepted.
    pub fn selected_prompt(&self) -> String {
        let option = &self.options[self.selected];
        match self.prompt.as_deref() {
            Some(prompt) => format!("{prompt}: {option}"),
            None => option.clone(),
        }
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub(crate) fn is_pending(&self) -> bool {
        !self.submitted
    }

    pub(crate) fn take_selected_prompt(&mut self) -> Option<String> {
        if self.submitted {
            return None;
        }
        self.submitted = true;
        Some(self.selected_prompt())
    }
}

impl BlockContent for ChoiceBlock {
    fn output(&self, _ctx: &BlockContext) -> BlockOutput {
        let mut text = String::new();
        if let Some(prompt) = &self.prompt {
            text.push_str(prompt);
            text.push('\n');
        }
        for (index, option) in self.options.iter().enumerate() {
            text.push_str(&format!("{}. {}\n", index + 1, option));
        }
        text.pop();

        let mut output = BlockOutput::plain(&text);
        if self.prompt.is_some() {
            output.lines[0].content.style = Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD);
        }
        let selected_line = self.selected + usize::from(self.prompt.is_some());
        output.lines[selected_line].content.style = Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        output
    }

    fn accent(&self, _ctx: &BlockContext) -> Option<AccentStyle> {
        Some(AccentStyle::static_color(Color::Cyan))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_ignores_non_options_and_rejects_empty_choices() {
        let parsed = ChoiceBlock::parse("prompt: Pick\nignored\n- One\n- Two\n").unwrap();
        assert_eq!(parsed.prompt.as_deref(), Some("Pick"));
        assert_eq!(parsed.options, ["One", "Two"]);
        assert!(ChoiceBlock::parse("prompt: Pick\nignored\n").is_none());
    }

    #[test]
    fn render_block_moves_and_selects_choice() {
        let mut block = crate::scrollback::RenderBlock::Choice(
            ChoiceBlock::parse("prompt: Pick\n- One\n- Two\n").unwrap(),
        );

        assert!(block.move_choice_selection(1));
        let crate::scrollback::RenderBlock::Choice(choice) = &block else {
            unreachable!()
        };
        assert_eq!(choice.selected_index(), 1);
        assert_eq!(block.selected_choice_prompt().as_deref(), Some("Pick: Two"));

        assert!(block.move_choice_selection(1));
        assert_eq!(
            block.take_selected_choice_prompt().as_deref(),
            Some("Pick: One")
        );
        assert_eq!(block.take_selected_choice_prompt(), None);
        assert!(!block.move_choice_selection(1));
    }
}
