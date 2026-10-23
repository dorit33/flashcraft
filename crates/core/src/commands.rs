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
    /// JSON Schema for the params, exposed as `inputSchema` by the MCP server.
    pub input_schema: &'static str,
    /// Whether running it changes a device (UI asks for confirmation).
    pub destructive: bool,
}

const SPECS: &[CommandSpec] = &[
    CommandSpec {
        id: "drives.list",
        label: "List removable drives",
        summary: "Enumerate candidate target devices with size, vendor and mount state.",
        params_doc: "{}",
        input_schema: r#"{"type":"object","properties":{},"additionalProperties":false}"#,
        destructive: false,
    },
    CommandSpec {
        id: "image.inspect",
        label: "Inspect image",
        summary: "Detect image kind (ISO 9660, disk image, compressed) and size.",
        params_doc: r#"{"image": "path/to/image.iso"}"#,
        input_schema: r#"{"type":"object","properties":{"image":{"type":"string","description":"path to the image file"}},"required":["image"],"additionalProperties":false}"#,
        destructive: false,
    },
    CommandSpec {
        id: "image.checksum",
        label: "Compute image checksums",
        summary: "Compute MD5, SHA-1 and SHA-256 digests of an image file.",
        params_doc: r#"{"image": "path/to/image.iso"}"#,
        input_schema: r#"{"type":"object","properties":{"image":{"type":"string","description":"path to the image file"}},"required":["image"],"additionalProperties":false}"#,
        destructive: false,
    },
    CommandSpec {
        id: "media.write",
        label: "Write image to drive",
        summary: "Write the image (or decompressed stream) to a removable drive, then sync.",
        params_doc: r#"{"image": "path/to/image.iso", "drive": "/dev/sdX"}"#,
        input_schema: r#"{"type":"object","properties":{"image":{"type":"string","description":"path to the image file"},"drive":{"type":"string","description":"/dev/sdX or kernel name"},"label":{"type":"string","description":"volume label"},"confirm":{"type":"boolean","description":"must be true — overwrites the whole drive; also needs root"},"persistence_mib":{"type":"integer","minimum":0,"description":"extra ext4 persistent partition, MiB"},"bad_blocks":{"type":"integer","minimum":0,"maximum":4,"description":"bad-block scan passes after writing"},"old_bios_fixes":{"type":"boolean"},"verify_after":{"type":"boolean"},"extended_label":{"type":"boolean","description":"write autorun.inf label files"}},"required":["image","drive","confirm"],"additionalProperties":false}"#,
        destructive: true,
    },
    CommandSpec {
        id: "media.verify",
        label: "Verify written drive",
        summary: "Read the drive back and compare it against the image.",
        params_doc: r#"{"image": "path/to/image.iso", "drive": "/dev/sdX"}"#,
        input_schema: r#"{"type":"object","properties":{"image":{"type":"string","description":"path to the image file"},"drive":{"type":"string","description":"/dev/sdX or kernel name"},"label":{"type":"string","description":"volume label used when writing"}},"required":["image","drive"],"additionalProperties":false}"#,
        destructive: false,
    },
    CommandSpec {
        id: "media.format",
        label: "Format removable drive",
        summary:
            "Partition (MBR/GPT) and format (FAT32/NTFS/exFAT/UDF/ext2/3/4) a removable drive.",
        params_doc: r#"{"drive": "/dev/sdX", "scheme": "MBR|GPT", "fs": "FAT32|NTFS|exFAT|UDF|ext2|ext3|ext4"}"#,
        input_schema: r#"{"type":"object","properties":{"drive":{"type":"string","description":"/dev/sdX or kernel name"},"confirm":{"type":"boolean","description":"must be true — erases the whole drive; also needs root"},"scheme":{"type":"string","enum":["MBR","GPT"]},"fs":{"type":"string","enum":["FAT32","NTFS","exFAT","UDF","ext2","ext3","ext4"]},"cluster":{"type":"integer","minimum":512,"description":"cluster size in bytes"},"label":{"type":"string","description":"volume label"},"quick":{"type":"boolean","description":"quick format (default true)"},"extended_label":{"type":"boolean"},"bad_blocks":{"type":"integer","minimum":0,"maximum":4},"old_bios_fixes":{"type":"boolean"}},"required":["drive","confirm"],"additionalProperties":false}"#,
        destructive: true,
    },
    CommandSpec {
        id: "media.badblocks",
        label: "Scan drive for bad blocks",
        summary: "Scan a removable drive for bad sectors over 1..=4 passes.",
        params_doc: r#"{"drive": "/dev/sdX", "passes": 1}"#,
        input_schema: r#"{"type":"object","properties":{"drive":{"type":"string","description":"/dev/sdX or kernel name"},"passes":{"type":"integer","minimum":1,"maximum":4}},"required":["drive"],"additionalProperties":false}"#,
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
