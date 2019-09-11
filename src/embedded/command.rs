use embedded::ValvePinNumber;

pub enum Command {
    Open(ValvePinNumber),
    Close(ValvePinNumber),
}
