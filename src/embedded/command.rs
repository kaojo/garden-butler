use embedded::ValvePinNumber;

#[derive(Debug)]
pub enum LayoutCommand {
    Open(ValvePinNumber),
    Close(ValvePinNumber),
}
