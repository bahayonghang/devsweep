use anyhow::Result;
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::Alignment,
    widgets::{Block, Paragraph},
};

pub fn run() -> Result<()> {
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(render_placeholder)?;
    Ok(())
}

pub fn render_placeholder(frame: &mut Frame<'_>) {
    let widget =
        Paragraph::new("Foundation build\n\nScanning and cleanup actions are not implemented yet.")
            .block(Block::bordered().title("devsweep"))
            .alignment(Alignment::Center);

    frame.render_widget(widget, frame.area());
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::render_placeholder;

    #[test]
    fn placeholder_renders_with_test_backend() {
        let backend = TestBackend::new(80, 8);
        let mut terminal = Terminal::new(backend).expect("test terminal");

        terminal
            .draw(render_placeholder)
            .expect("placeholder renders");
    }
}
