use uefi::{
    Result,
    proto::console::text::{Input, Key, ScanCode},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    Up,
    Down,
    Confirm,
    Cancel,
}

pub fn read_event(input: &mut Input) -> Result<Option<InputEvent>> {
    let Some(key) = input.read_key()? else {
        return Ok(None);
    };

    let event = match key {
        Key::Special(ScanCode::UP) => Some(InputEvent::Up),

        Key::Special(ScanCode::DOWN) => Some(InputEvent::Down),

        Key::Special(ScanCode::ESCAPE) => Some(InputEvent::Cancel),

        Key::Printable(character) if character == '\r' || character == '\n' => {
            Some(InputEvent::Confirm)
        }

        _ => None,
    };

    Ok(event)
}
