//! Native IDataObject format and owned-memory regressions.

use super::*;
use std::cell::Cell;
use windows::Win32::Foundation::HGLOBAL;
use windows::Win32::System::Com::{STATFLAG_NONAME, STATSTG, STREAM_SEEK_CUR, TYMED_ISTREAM};

fn controlled_medium() -> STGMEDIUM {
    STGMEDIUM {
        tymed: TYMED_HGLOBAL.0 as u32,
        u: STGMEDIUM_0 {
            hGlobal: HGLOBAL(std::ptr::null_mut()),
        },
        pUnkForRelease: ManuallyDrop::new(None),
    }
}

fn count_releases(release: BOOL, repetitions: usize) -> usize {
    let release_count = Cell::new(0);
    let medium = controlled_medium();
    for _ in 0..repetitions {
        finish_set_data(Ok(()), &raw const medium, release, |_| {
            release_count.set(release_count.get() + 1);
        })
        .expect("controlled SetData operation should succeed");
    }
    release_count.get()
}

#[test]
fn transferred_medium_is_released_once_per_successful_set_data() {
    assert_eq!(count_releases(BOOL::from(true), 8), 8);
}

#[test]
fn caller_owned_medium_is_never_released() {
    assert_eq!(count_releases(BOOL::from(false), 8), 0);
}

#[test]
fn failed_set_data_does_not_take_medium_ownership() {
    let release_count = Cell::new(0);
    let medium = controlled_medium();
    let result = finish_set_data(
        Err(windows::core::Error::from(E_INVALIDARG)),
        &raw const medium,
        BOOL::from(true),
        |_| release_count.set(release_count.get() + 1),
    );

    assert!(result.is_err());
    assert_eq!(release_count.get(), 0);
}

#[test]
fn drop_effect_medium_read_rejects_unlocked_null_handle() {
    let medium = STGMEDIUM {
        tymed: TYMED_HGLOBAL.0 as u32,
        u: STGMEDIUM_0 {
            hGlobal: HGLOBAL(std::ptr::null_mut()),
        },
        pUnkForRelease: ManuallyDrop::new(None),
    };

    assert!(drop_effect_from_medium(&medium).is_err());
}

#[test]
fn url_data_object_routes_only_the_single_url_format() {
    let object = ExternalDragDataObject::url(String::from("https://example.test/drag"))
        .expect("URL data object");
    let mut wrong_lindex = object.format;
    wrong_lindex.lindex = 0;

    assert!(object.matches_format(&object.format));
    assert!(!object.matches_format(&wrong_lindex));
    assert!(!object.matches_format(&build_text_format()));
}

#[test]
fn mime_data_object_advertises_stream_and_rejects_hglobal_only_requests() {
    let object = ExternalDragDataObject::mime("application/x-radiant".into(), Vec::new()).unwrap();
    assert_eq!(object.format.tymed, TYMED_ISTREAM.0 as u32);
    assert!(object.matches_format(&object.format));
    let mut unsupported = object.format;
    unsupported.tymed = TYMED_HGLOBAL.0 as u32;
    assert!(!object.matches_format(&unsupported));
    unsupported = object.format;
    unsupported.lindex = 0;
    assert!(!object.matches_format(&unsupported));
}

#[test]
fn mime_data_object_streams_have_exact_bytes_and_independent_cursors() {
    for expected in [Vec::new(), vec![0_u8, 1, 0, 255]] {
        let object =
            ExternalDragDataObject::mime("application/x-radiant".into(), expected.clone()).unwrap();
        let mut first = object.fill_medium(&object.format).unwrap();
        let mut second = object.fill_medium(&object.format).unwrap();
        {
            let stream = unsafe { first.u.pstm.as_ref() }.unwrap();
            let mut stat = STATSTG::default();
            unsafe { stream.Stat(&raw mut stat, STATFLAG_NONAME) }.unwrap();
            assert_eq!(stat.cbSize, expected.len() as u64);
            let mut buffer = vec![0; expected.len() + 1];
            let mut read = 0;
            unsafe {
                stream
                    .Read(
                        buffer.as_mut_ptr().cast(),
                        buffer.len() as u32,
                        Some(&raw mut read),
                    )
                    .ok()
            }
            .unwrap();
            assert_eq!(read as usize, expected.len());
            assert_eq!(&buffer[..read as usize], expected.as_slice());
            let other = unsafe { second.u.pstm.as_ref() }.unwrap();
            let mut position = u64::MAX;
            unsafe { other.Seek(0, STREAM_SEEK_CUR, Some(&raw mut position)) }.unwrap();
            assert_eq!(position, 0);
        }
        unsafe {
            ReleaseStgMedium(&raw mut first);
            ReleaseStgMedium(&raw mut second);
        }
    }
}
