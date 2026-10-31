use super::*;
use crate::host_build::Error;
use std::{io, path::PathBuf};

#[test]
fn every_host_build_variant_has_a_message() {
    for program in ["herdr-automate", "herdr-remote-control"] {
        assert_localized(&[
            Error::Binary {
                program,
                path: PathBuf::from("/opt/herdr/bin").join(program),
                source: io::Error::other("gone"),
            },
            Error::Checksum,
            Error::Platform("Plan9 mips".into()),
        ]);
    }
}
