use std::mem;
use winapi::shared::guiddef::GUID;
use winapi::shared::winerror::{NOERROR, S_OK};
use winapi::um::combaseapi::{CLSIDFromString, CoCreateGuid, StringFromGUID2};

const GUID_STRING_CHARACTERS: usize = 38;
const GUID_N_CHARACTERS: usize = 32;

pub struct Guid {
    clsid: GUID,
}

impl Guid {
    /// Returns a `CLSID` which is a globally unique identifier
    /// that identifies a COM class object.
    /// Also known as GUID/UUID to others.
    pub fn clsid(&self) -> GUID {
        return self.clsid;
    }

    /// Creates a new `Guid` structure which has a backing `CLSID`.A
    /// The Guid is UUID v4 compatible.
    pub fn new() -> Option<Guid> {
        if let Some(clsid) = generate_guid() {
            return Some(Guid { clsid });
        }
        None
    }

    /// Returns a string representation of the value of this instance of the Guid structure.
    /// The following table shows the accepted format specifiers for the format parameter.
    ///
    /// | Specifier |                 Format of return value |
    /// |-----------|---------------------------------------:|
    /// |    `N`    |      `00000000000000000000000000000000`|
    /// |    `D`    |  `00000000-0000-0000-0000-000000000000`|
    /// |    `B`    |`{00000000-0000-0000-0000-000000000000}`|
    /// |    `P`    |`(00000000-0000-0000-0000-000000000000)`|
    pub fn format(&self, specifier: &str) -> Option<String> {
        use std::os::raw::c_int;
        let mut s: [u16; GUID_STRING_CHARACTERS + 1] = unsafe { mem::uninitialized() };
        let len = unsafe {
            StringFromGUID2(
                &(self.clsid).Data1 as *const _ as *mut _,
                s.as_mut_ptr(),
                s.len() as c_int,
            )
        };
        if len <= 0 {
            return None;
        }
        // len is number of characters, including the null terminator
        let s = &s[..len as usize - 1];
        let guid_string = String::from_utf16_lossy(&s)
            .trim_end_matches(0x00 as char)
            .to_string();
        // format the GUID string according to the specifier
        match specifier {
            "B" | "" => Some(guid_string),
            "N" => Some(
                guid_string
                    .replace('{', "")
                    .replace('}', "")
                    .replace('-', ""),
            ),
            "D" => Some(guid_string.replace('{', "").replace('}', "")),
            "P" => Some(guid_string.replace('{', "(").replace('}', ")")),
            _ => unimplemented!(),
        }
    }

    /// Reads and creates a `Guid` structure from a string using `CLSIDFromString`
    /// It will automatically add braces (`{}`)  if they are missing.
    pub fn from_str(guid_str: &str) -> Option<Guid> {
        // the length of a GUID formatted with `N`
        if guid_str.len() < GUID_N_CHARACTERS {
            return None;
        }
        let mut clsid = unsafe { mem::uninitialized() };

        // https://i.imgur.com/JA1y4DR.png
        let formatted = if !guid_str.starts_with('{') && !guid_str.contains('-') {
            format!(
                "{{{}-{}-{}-{}-{}}}",
                &guid_str[0..8],
                &guid_str[8..12],
                &guid_str[12..16],
                &guid_str[16..20],
                &guid_str[20..GUID_N_CHARACTERS]
            )
        } else {
            guid_str.to_string()
        };
        log::info!("what {}", formatted);
        let s: Vec<_> = formatted.encode_utf16().chain(Some(0)).collect();
        unsafe {
            let ret = CLSIDFromString(s.as_ptr(), &mut clsid);
            if ret != NOERROR {
                return None;
            }
        };
        Some(Guid { clsid })
    }
}

/// generates a new GUID via `CoCreateGuid`
fn generate_guid() -> Option<GUID> {
    let mut result: GUID = GUID {
        Data1: 0x0,
        Data2: 0x0,
        Data3: 0x0,
        Data4: [0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x01],
    };
    if unsafe { CoCreateGuid(&mut result as *mut GUID) } != S_OK {
        return None;
    }
    Some(result)
}

#[cfg(test)]
mod test {
    use super::Guid;
    #[test]
    fn with_braces() {
        let clsid = "{FA19F1DF-3226-441A-BA5B-40F6EB8AB6B1}";
        let guid = Guid::from_str(clsid).unwrap().format("B").unwrap();
        assert_eq!(guid, clsid);
    }

    #[test]
    fn without_braces() {
        let clsid = "FA19F1DF3226441ABA5B40F6EB8AB6B1";
        let guid = Guid::from_str(clsid).unwrap().format("N").unwrap();
        assert_eq!(guid, clsid);
    }

    #[test]
    fn format_error() {
        let clsid = "Pork Chop Sandwiches";
        assert_eq!(Guid::from_str(clsid).is_none(), true);
    }
}
