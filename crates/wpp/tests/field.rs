// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

use std::ffi::CString;

use wpp::{IntoWppField, WppField};

struct CustomField([u8; 4]);

impl WppField for CustomField {
    const FORMAT_SPEC: &'static str = "!lu!";
    const TYPE_NAME: &'static str = "ItemLong";

    fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(wpp::WppField)]
#[repr(u8)]
enum ByteState {
    Unknown,
    Starting,
    Ready,
}

#[derive(wpp::WppField)]
#[repr(u16)]
enum ShortState {
    Unknown,
    Warning = 10,
    Failed,
}

#[derive(wpp::WppField)]
#[repr(u32)]
enum LongState {
    Offline,
    Online,
    Recovering = 100_000,
}

#[test]
fn custom_field_passes_through_without_copy() {
    let field: CustomField = CustomField(42_u32.to_ne_bytes()).into_wpp_field();
    assert_eq!(field.as_bytes(), &42_u32.to_ne_bytes());
    assert_eq!(CustomField::TYPE_NAME, "ItemLong");
    assert_eq!(CustomField::FORMAT_SPEC, "!lu!");
}

#[test]
fn references_borrow_field_bytes_not_pointer_bytes() {
    let field = CustomField(42_u32.to_ne_bytes());
    let borrowed: &CustomField = (&field).into_wpp_field();
    let nested: &&CustomField = (&borrowed).into_wpp_field();

    assert_eq!(nested.as_bytes(), &42_u32.to_ne_bytes());
    assert_eq!(borrowed.as_bytes().as_ptr(), field.0.as_ptr());
    assert_eq!(
        <&CustomField as WppField>::TYPE_NAME,
        CustomField::TYPE_NAME
    );
    assert_eq!(
        <&&CustomField as WppField>::FORMAT_SPEC,
        CustomField::FORMAT_SPEC
    );
}

#[test]
fn derived_byte_enum_does_not_require_copy() {
    for (state, expected) in [
        (ByteState::Unknown, 0),
        (ByteState::Starting, 1),
        (ByteState::Ready, 2),
    ] {
        assert_eq!((&state).into_wpp_field().as_bytes(), &[expected]);
        let field: ByteState = state.into_wpp_field();
        assert_eq!(field.as_bytes(), &[expected]);
    }
    assert_eq!(ByteState::TYPE_NAME, "ItemListByte(Unknown,Starting,Ready)");
    assert_eq!(ByteState::FORMAT_SPEC, "!s!");
}

#[test]
fn derived_short_enum_preserves_sparse_and_implicit_discriminants() {
    for (state, expected) in [
        (ShortState::Unknown, 0_u16),
        (ShortState::Warning, 10),
        (ShortState::Failed, 11),
    ] {
        assert_eq!(state.as_bytes(), &expected.to_ne_bytes());
    }
    assert_eq!(
        ShortState::TYPE_NAME,
        "ItemListShort(Unknown,Warning=10,Failed)"
    );
    assert_eq!(ShortState::FORMAT_SPEC, "!s!");
}

#[test]
fn derived_long_enum_preserves_discriminants() {
    for (state, expected) in [
        (LongState::Offline, 0_u32),
        (LongState::Online, 1),
        (LongState::Recovering, 100_000),
    ] {
        assert_eq!(state.as_bytes(), &expected.to_ne_bytes());
    }
    assert_eq!(
        LongState::TYPE_NAME,
        "ItemListLong(Offline,Online,Recovering=100000)"
    );
    assert_eq!(LongState::FORMAT_SPEC, "!s!");
}

#[test]
fn strings_keep_their_null_terminated_encoding() {
    assert_eq!(WppField::as_bytes(&"hello".into_wpp_field()), b"hello\0");
    assert_eq!(
        WppField::as_bytes(&String::from("hello").into_wpp_field()),
        b"hello\0"
    );
    assert_eq!(WppField::as_bytes(&c"hello".into_wpp_field()), b"hello\0");

    let string = CString::new("hello").unwrap();
    assert_eq!(WppField::as_bytes(&(&string).into_wpp_field()), b"hello\0");
    assert_eq!(WppField::as_bytes(&string.into_wpp_field()), b"hello\0");
}

#[test]
fn borrowed_primitive_preserves_encoding() {
    let value = 42_u32;
    let field: &u32 = (&value).into_wpp_field();
    assert_eq!(field.as_bytes(), &value.to_ne_bytes());
}
