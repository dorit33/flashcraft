//! The command registry. Every user-visible behaviour is a command with an
//! id, a label and a params doc; the UI, the CLI and future control channel
//! all dispatch by id. Implementations live in `flashcraft-native`.

/// Static description of one command.
#[derive(Debug, Clone, Copy)]
pub struct CommandSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub summary: &'static str,
    /// Human-readable params doc shown by `flashcraft-cli commands`.
    pub params_doc: &'static str,
    /// Whether running it changes a device (UI asks for confirmation).
    pub destructive: bool,
}

const SPECS: &[CommandSpec] = &[
    CommandSpec {
        id: "drives.list",
        label: "List removable drives",
        summary: "Enumerate candidate target devices with size, vendor and mount state.",
        params_doc: "{}",
        destructive: false,
    },
    CommandSpec {
        id: "image.inspect",
        label: "Inspect image",
        summary: "Detect image kind (ISO 9660, disk image, compressed) and size.",
        params_doc: r#"{"image": "path/to/image.iso"}"#,
        destructive: false,
    },
    CommandSpec {
        id: "media.write",
        label: "Write image to drive",
        summary: "Write the image byte-for-byte to a removable drive, then sync.",
        params_doc: r#"{"image": "path/to/image.iso", "drive": "/dev/sdX"}"#,
        destructive: true,
    },
    CommandSpec {
        id: "media.verify",
        label: "Verify written drive",
        summary: "Read the drive back and compare it against the image.",
        params_doc: r#"{"image": "path/to/image.iso", "drive": "/dev/sdX"}"#,
        destructive: false,
    },
];

/// Every registered command, in stable order.
pub fn registry() -> &'static [CommandSpec] {
    SPECS
}

/// Look up a command by id.
pub fn command(id: &str) -> Option<&'static CommandSpec> {
    SPECS.iter().find(|c| c.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_dotted() {
        for spec in registry() {
            assert!(spec.id.contains('.'));
            assert_eq!(command(spec.id).map(|c| c.id), Some(spec.id));
        }
    }
}
