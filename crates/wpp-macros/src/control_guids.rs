// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

//! Implementation of `wpp_control_guids!` proc macro.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Ident,
    LitInt,
    LitStr,
    Result,
    Token,
    braced,
    parse::{Parse, ParseStream},
};

// ─── AST ────────────────────────────────────────────────────────────────────

struct ProviderDecl {
    name: Ident,
    guid_str: String,
    guid_parts: GuidParts,
    keywords: Vec<KeywordDecl>,
}

struct KeywordDecl {
    name: Ident,
    bit_position: u32,
}

#[derive(Clone)]
pub(super) struct GuidParts {
    pub(super) d1: u32,
    pub(super) d2: u16,
    pub(super) d3: u16,
    pub(super) d4: [u8; 8],
}

struct ControlGuidsInput {
    providers: Vec<ProviderDecl>,
}

// ─── Parsing ────────────────────────────────────────────────────────────────

impl Parse for ControlGuidsInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut providers = Vec::new();
        while !input.is_empty() {
            providers.push(input.parse::<ProviderDecl>()?);
        }
        if providers.is_empty() {
            return Err(input.error("expected at least one provider declaration"));
        }
        Ok(ControlGuidsInput { providers })
    }
}

impl Parse for ProviderDecl {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        if name == "wpp_tracing" || name == "__wpp_ifr" {
            return Err(syn::Error::new(
                name.span(),
                "provider name is reserved for WPP lifecycle support",
            ));
        }
        let guid_str = parse_guid(input)?;
        let guid_parts =
            parse_guid_parts(&guid_str).map_err(|e| input.error(format!("invalid GUID: {}", e)))?;

        let content;
        braced!(content in input);
        let mut keywords = Vec::new();
        let mut next_auto_bit: u32 = 0;

        while !content.is_empty() {
            let kw_name: Ident = content.parse()?;
            let bit_pos = if content.peek(Token![=]) {
                content.parse::<Token![=]>()?;
                let lit: LitInt = content.parse()?;
                let pos = lit.base10_parse::<u32>()?;
                next_auto_bit = pos + 1;
                pos
            } else {
                let pos = next_auto_bit;
                next_auto_bit += 1;
                pos
            };
            keywords.push(KeywordDecl {
                name: kw_name,
                bit_position: bit_pos,
            });
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(ProviderDecl {
            name,
            guid_str,
            guid_parts,
            keywords,
        })
    }
}

fn parse_guid(input: ParseStream) -> Result<String> {
    // The GUID must be a string literal. Bare GUIDs are split into Rust
    // tokens before the macro runs, which fails for segments such as
    // `34746ee9` that look like malformed numeric literals.
    let lit: LitStr = input.parse().map_err(|error| {
        syn::Error::new(
            error.span(),
            "expected a quoted control GUID, e.g. \"84bdb2e9-829e-41b3-b891-02f454bc2bd7\"",
        )
    })?;
    validate_guid(&lit.value()).map_err(|message| syn::Error::new(lit.span(), message))
}

fn validate_guid(guid: &str) -> core::result::Result<String, String> {
    let segs: Vec<&str> = guid.split('-').collect();
    if segs.len() != 5 {
        return Err(format!(
            "GUID must have 5 dash-separated segments, found {}: '{}'",
            segs.len(),
            guid
        ));
    }
    let expected_lens = [8, 4, 4, 4, 12];
    for (i, (seg, &exp)) in segs.iter().zip(&expected_lens).enumerate() {
        if seg.len() != exp {
            return Err(format!(
                "GUID segment {} has length {} (expected {}): '{}'",
                i,
                seg.len(),
                exp,
                guid
            ));
        }
        if !seg.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!(
                "GUID segment {} contains non-hex characters: '{}'",
                i, seg
            ));
        }
    }
    Ok(guid.to_string())
}

pub(super) fn parse_guid_parts(s: &str) -> core::result::Result<GuidParts, String> {
    let segs: Vec<&str> = s.split('-').collect();
    if segs.len() != 5 {
        return Err("expected 5 segments".into());
    }
    let d1 = u32::from_str_radix(segs[0], 16).map_err(|e| e.to_string())?;
    let d2 = u16::from_str_radix(segs[1], 16).map_err(|e| e.to_string())?;
    let d3 = u16::from_str_radix(segs[2], 16).map_err(|e| e.to_string())?;
    let d4_hi = u16::from_str_radix(segs[3], 16).map_err(|e| e.to_string())?;
    let d4_lo = u64::from_str_radix(segs[4], 16).map_err(|e| e.to_string())?;
    let mut d4 = [0u8; 8];
    d4[0] = (d4_hi >> 8) as u8;
    d4[1] = d4_hi as u8;
    d4[2] = (d4_lo >> 40) as u8;
    d4[3] = (d4_lo >> 32) as u8;
    d4[4] = (d4_lo >> 24) as u8;
    d4[5] = (d4_lo >> 16) as u8;
    d4[6] = (d4_lo >> 8) as u8;
    d4[7] = d4_lo as u8;
    Ok(GuidParts { d1, d2, d3, d4 })
}

// ─── Code generation ────────────────────────────────────────────────────────

pub fn generate(input: TokenStream) -> Result<TokenStream> {
    let parsed: ControlGuidsInput = syn::parse2(input)?;
    let mut output = TokenStream::new();

    validate_unique_keywords(&parsed.providers)?;

    output.extend(generate_control_guid_array(&parsed.providers));
    for (idx, provider) in parsed.providers.iter().enumerate() {
        output.extend(generate_provider_module(provider, idx));
    }
    output.extend(generate_wpp_flag_enum(&parsed.providers));
    output.extend(generate_unified_trace_macro(&parsed.providers));
    output.extend(generate_level_macro());
    output.extend(generate_lifecycle_module(&parsed.providers));
    if cfg!(feature = "ifr") {
        output.extend(generate_ifr_module(&parsed.providers));
    }
    Ok(output)
}

/// Generates a global static array of control GUIDs for all providers.
///
/// Each provider module references its GUID via
/// `super::WPP_CONTROL_GUIDS[CB_INDEX]`.
fn generate_control_guid_array(providers: &[ProviderDecl]) -> TokenStream {
    let num_providers = providers.len();
    let num_providers_lit = proc_macro2::Literal::usize_unsuffixed(num_providers);

    let guid_elements: Vec<TokenStream> = providers
        .iter()
        .map(|p| {
            let gp = &p.guid_parts;
            let (d1, d2, d3) = (gp.d1, gp.d2, gp.d3);
            let d4_tokens: Vec<TokenStream> = gp
                .d4
                .iter()
                .map(|b| {
                    let b = *b;
                    quote!(#b)
                })
                .collect();
            quote! {
                ::wpp::GUID { data1: #d1, data2: #d2, data3: #d3, data4: [#(#d4_tokens),*] }
            }
        })
        .collect();

    quote! {
        /// Global static array of control GUIDs for all providers.
        static WPP_CONTROL_GUIDS: [::wpp::GUID; #num_providers_lit] = [
            #(#guid_elements),*
        ];
    }
}

fn generate_provider_module(p: &ProviderDecl, idx: usize) -> TokenStream {
    let mod_name = &p.name;

    let kw_consts: Vec<TokenStream> = p
        .keywords
        .iter()
        .map(|kw| {
            let name = &kw.name;
            let value = 1u64 << kw.bit_position;
            quote! { #[allow(non_upper_case_globals)] pub const #name: u64 = #value; }
        })
        .collect();

    let provider_name_str = p.name.to_string();
    let guid_str = &p.guid_str;
    // TMC annotation lists each flag by name only (no bit suffix).
    let kw_annotation_strings: Vec<String> =
        p.keywords.iter().map(|kw| kw.name.to_string()).collect();

    let cb_index_lit = proc_macro2::Literal::usize_unsuffixed(idx);

    quote! {
        #[allow(non_snake_case)]
        pub mod #mod_name {
            /// Reference to this provider's control GUID in the global array.
            #[inline]
            pub fn control_guid() -> &'static ::wpp::GUID {
                &super::WPP_CONTROL_GUIDS[CB_INDEX]
            }
            /// Index of this provider's control block in the IFR CB array.
            pub const CB_INDEX: usize = #cb_index_lit;
            #(#kw_consts)*
            #[doc(hidden)]
            pub const __WPP_NO_KEYWORD: u64 = 0;
            pub static STATE: ::wpp::ProviderState = ::wpp::ProviderState::new();

            /// Initializes this provider: emits PDB annotation and registers
            /// with ETW.
            ///
            /// # Safety
            ///
            /// When the `ifr` feature is enabled, IFR must already have been
            /// initialized through the generated `wpp_tracing` module.
            /// The caller must ensure `clean_up()` is called before the
            /// module containing this provider is unloaded.
            pub unsafe fn init() {
                struct Args;
                impl ::core::os::windows::CodeViewAnnotationArgs for Args {
                    const ARGS: &'static [&'static str] = &[
                        "TMC:", #guid_str, #provider_name_str,
                        #(#kw_annotation_strings),*
                    ];
                }
                // Emit this provider's `TMC:` control-GUID annotation (control
                // GUID -> provider name + flag names). It is keyed under the
                // control GUID, which under Option B is distinct from the
                // per-module decode GUIDs that key the TMF records, so TMC and
                // TMF annotations never share a GUID and cannot collide.
                ::core::os::windows::codeview_annotation::<Args>();
                if STATE.init_state.compare_exchange(
                    ::wpp::provider::UNINITIALIZED,
                    ::wpp::provider::INITIALIZING,
                    ::core::sync::atomic::Ordering::Acquire,
                    ::core::sync::atomic::Ordering::Relaxed,
                ).is_err() {
                    return;
                }
                // TODO: [P2] Handle registration error here.
                let (_, handle) = unsafe { ::wpp::etw::register(
                    control_guid(),
                    ::core::option::Option::Some(::wpp::provider::enable_callback),
                    &STATE as *const ::wpp::ProviderState as *mut ::core::ffi::c_void,
                ) };
                STATE.reg_handle.store(handle, ::core::sync::atomic::Ordering::Relaxed);
                // ModernWpp (WPPv3): mark this Crimson provider as a WPP
                // trace-message provider so the kernel stamps the WPP header
                // flag and tracks the PDB DebugId for TMF decoding, and opt in
                // to descriptor typing so the per-module decode-GUID descriptor
                // (Option B) is honoured. Best-effort — ignored on kernels
                // without the feature. Enablement stays keyed on this control
                // GUID; each event carries its module's decode GUID separately.
                unsafe { ::wpp::etw::enable_modern_wpp(handle) };
                STATE.init_state.store(
                    ::wpp::provider::INITIALIZED,
                    ::core::sync::atomic::Ordering::Release,
                );
            }

            pub fn clean_up() {
                if STATE.init_state.compare_exchange(
                    ::wpp::provider::INITIALIZED,
                    ::wpp::provider::UNINITIALIZED,
                    ::core::sync::atomic::Ordering::Acquire,
                    ::core::sync::atomic::Ordering::Relaxed,
                ).is_err() {
                    return;
                }
                STATE.enabled_level.store(0, ::core::sync::atomic::Ordering::Relaxed);
                STATE.enabled_keywords.store(0, ::core::sync::atomic::Ordering::Relaxed);
                let handle = STATE.reg_handle.swap(0, ::core::sync::atomic::Ordering::Relaxed);
                if handle != 0 { unsafe { ::wpp::etw::unregister(handle) }; }
            }
        }
    }
}

fn validate_unique_keywords(providers: &[ProviderDecl]) -> Result<()> {
    let mut seen = std::collections::HashMap::<String, String>::new();
    for p in providers {
        for kw in &p.keywords {
            let name = kw.name.to_string();
            if let Some(prev_provider) = seen.insert(name.clone(), p.name.to_string()) {
                return Err(syn::Error::new(
                    kw.name.span(),
                    format!(
                        "keyword '{}' is defined in both '{}' and '{}'; keywords must be unique \
                         across all providers",
                        name, prev_provider, p.name
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// Generates the `TraceLevel` and `WppFlag` enums.
///
/// `WppFlag` follows the reference implementation: each variant carries
/// `(control_idx, flag_idx)` and provides `control_index()`, `flag_index()`,
/// `as_tuple()`, `by_name()`, `all()`, `count()`.
fn generate_wpp_flag_enum(providers: &[ProviderDecl]) -> TokenStream {
    let has_flags = providers.iter().any(|p| !p.keywords.is_empty());

    struct VariantInfo {
        variant_ident: Ident,
        flag_name: String,
        control_idx: usize,
        flag_idx: usize,
    }

    let variants: Vec<VariantInfo> = providers
        .iter()
        .enumerate()
        .flat_map(|(control_idx, p)| {
            p.keywords
                .iter()
                .enumerate()
                .map(move |(flag_idx, kw)| VariantInfo {
                    variant_ident: kw.name.clone(),
                    flag_name: kw.name.to_string(),
                    control_idx,
                    flag_idx,
                })
        })
        .collect();

    let enum_variants: Vec<TokenStream> = variants
        .iter()
        .map(|v| {
            let ident = &v.variant_ident;
            quote! { #ident(usize, usize) }
        })
        .collect();

    let name_match_arms: Vec<TokenStream> = variants
        .iter()
        .map(|v| {
            let ident = &v.variant_ident;
            let name = &v.flag_name;
            let ctrl_idx = proc_macro2::Literal::usize_unsuffixed(v.control_idx);
            let flag_idx = proc_macro2::Literal::usize_unsuffixed(v.flag_idx);
            quote! { #name => ::core::option::Option::Some(Self::#ident(#ctrl_idx, #flag_idx)) }
        })
        .collect();

    let static_entries: Vec<TokenStream> = variants
        .iter()
        .map(|v| {
            let ident = &v.variant_ident;
            let ctrl_idx = proc_macro2::Literal::usize_unsuffixed(v.control_idx);
            let flag_idx = proc_macro2::Literal::usize_unsuffixed(v.flag_idx);
            quote! { Self::#ident(#ctrl_idx, #flag_idx) }
        })
        .collect();

    let control_idx_arms: Vec<TokenStream> = variants
        .iter()
        .map(|v| {
            let ident = &v.variant_ident;
            quote! { Self::#ident(ctrl, _) => *ctrl }
        })
        .collect();

    let flag_idx_arms: Vec<TokenStream> = variants
        .iter()
        .map(|v| {
            let ident = &v.variant_ident;
            quote! { Self::#ident(_, flag) => *flag }
        })
        .collect();

    let tuple_arms: Vec<TokenStream> = variants
        .iter()
        .map(|v| {
            let ident = &v.variant_ident;
            quote! { Self::#ident(ctrl, flag) => (*ctrl, *flag) }
        })
        .collect();

    let num_flags = variants.len();

    // TraceLevel is always generated; WppFlag only when flags exist
    let wpp_flag_block = if has_flags {
        quote! {
            #[allow(missing_docs)]
            #[allow(non_camel_case_types)]
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            #[repr(C)]
            pub enum WppFlag {
                #(#enum_variants),*
            }

            impl WppFlag {
                #[inline]
                pub const fn control_index(&self) -> usize {
                    match self { #(#control_idx_arms),* }
                }

                #[inline]
                pub const fn flag_index(&self) -> usize {
                    match self { #(#flag_idx_arms),* }
                }

                #[inline]
                pub const fn as_tuple(&self) -> (usize, usize) {
                    match self { #(#tuple_arms),* }
                }

                #[inline]
                pub fn by_name(name: &str) -> ::core::option::Option<Self> {
                    match name {
                        #(#name_match_arms,)*
                        _ => ::core::option::Option::None
                    }
                }

                #[inline]
                pub const fn all() -> &'static [Self] {
                    &[#(#static_entries),*]
                }

                #[inline]
                pub const fn count() -> usize { #num_flags }
            }
        }
    } else {
        quote! {}
    };

    quote! {
        // TODO: TraceLevel can be declared in wpp crate instead of being generated here
        #[allow(missing_docs)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(u8)]
        pub enum TraceLevel {
            None = 0,
            Critical = 1,
            Error = 2,
            Warning = 3,
            Information = 4,
            Verbose = 5,
            Reserved6 = 6,
            Reserved7 = 7,
            Reserved8 = 8,
            Reserved9 = 9,
        }

        impl TraceLevel {
            #[inline]
            pub const fn value(&self) -> u8 { *self as u8 }

            #[inline]
            pub const fn is_verbose_or_below(&self) -> bool {
                (*self as u8) <= (Self::Verbose as u8)
            }
        }

        #wpp_flag_block
    }
}

/// Generates a single unified `trace!` macro with one arm per keyword.
///
/// Each keyword is matched literally and routes to the correct provider.
/// A default arm (no keyword) routes to the first provider with keyword=0.
fn generate_unified_trace_macro(providers: &[ProviderDecl]) -> TokenStream {
    let dollar = proc_macro2::Punct::new('$', proc_macro2::Spacing::Alone);

    // Build one arm per keyword across all providers
    let mut arms: Vec<TokenStream> = Vec::new();
    for provider in providers {
        let mod_name = &provider.name;
        let provider_name_str = provider.name.to_string();
        let guid_str = &provider.guid_str;

        for kw in &provider.keywords {
            let kw_name = &kw.name;
            arms.push(quote! {
                (#dollar level:ident, #kw_name, #dollar fmt:literal #dollar(, #dollar arg:expr)*) => {{
                    const __WPP_LEVEL: u8 = __wpp_level_to_u8!(#dollar level);
                    // __wpp_trace_impl handles ETW (gated by is_enabled)
                    // and IFR (gated by IFR state auto_log_context) independently.
                    ::wpp::__wpp_trace_impl!(
                        @provider_mod #dollar crate::#mod_name,
                        @ifr_state #dollar crate::__wpp_ifr::STATE,
                        @provider_name #provider_name_str,
                        @guid #guid_str,
                        @level __WPP_LEVEL,
                        @level_name #dollar level,
                        @keyword #kw_name,
                        @fmt #dollar fmt
                        #dollar(, #dollar arg)*
                    );
                }};
            });
        }
    }

    // Default arm: no keyword → first provider, keyword = 0
    if let Some(first) = providers.first() {
        let mod_name = &first.name;
        let provider_name_str = first.name.to_string();
        let guid_str = &first.guid_str;

        arms.push(quote! {
            (#dollar level:ident, #dollar fmt:literal #dollar(, #dollar arg:expr)*) => {{
                const __WPP_LEVEL: u8 = __wpp_level_to_u8!(#dollar level);
                ::wpp::__wpp_trace_impl!(
                    @provider_mod #dollar crate::#mod_name,
                    @ifr_state #dollar crate::__wpp_ifr::STATE,
                    @provider_name #provider_name_str,
                    @guid #guid_str,
                    @level __WPP_LEVEL,
                    @level_name #dollar level,
                    @keyword __WPP_NO_KEYWORD,
                    @fmt #dollar fmt
                    #dollar(, #dollar arg)*
                );
            }};
        });
    }

    quote! {
        macro_rules! trace {
            #(#arms)*
        }
    }
}

fn generate_level_macro() -> TokenStream {
    quote! {
        macro_rules! __wpp_level_to_u8 {
            (CRITICAL) => { 1u8 };
            (ERROR)    => { 2u8 };
            (WARNING)  => { 3u8 };
            (INFO)     => { 4u8 };
            (VERBOSE)  => { 5u8 };
        }
    }
}

fn generate_lifecycle_module(providers: &[ProviderDecl]) -> TokenStream {
    let init_calls = providers.iter().map(|provider| {
        let name = &provider.name;
        quote! { unsafe { super::#name::init(); } }
    });
    let cleanup_calls = providers.iter().rev().map(|provider| {
        let name = &provider.name;
        quote! { super::#name::clean_up(); }
    });
    let (ifr_init_driver, ifr_init_app, ifr_cleanup) = if cfg!(feature = "ifr") {
        (
            quote! {
                unsafe { super::__wpp_ifr::init_driver(driver_object, registry_path); }
            },
            quote! {
                unsafe { super::__wpp_ifr::init(); }
            },
            quote! {
                unsafe { super::__wpp_ifr::cleanup(); }
            },
        )
    } else {
        (
            quote! { let _ = (driver_object, registry_path); },
            TokenStream::new(),
            TokenStream::new(),
        )
    };
    let app_init = if cfg!(feature = "user_mode") {
        quote! {
            /// Initializes application tracing and, when enabled, IFR.
            ///
            /// # Safety
            ///
            /// Initialization and cleanup must be externally serialized.
            /// Do not mix application and driver initialization in one active
            /// lifecycle. Call `cleanup` before unloading the tracing code.
            pub unsafe fn init() {
                #ifr_init_app
                unsafe { init_providers(); }
            }
        }
    } else {
        TokenStream::new()
    };

    quote! {
        /// Lifecycle operations for all providers declared by `wpp_control_guids!`.
        pub mod wpp_tracing {
            /// Initializes driver tracing and, when enabled, IFR.
            ///
            /// # Safety
            ///
            /// `driver_object` must be a non-null, valid native driver-object
            /// pointer for the selected mode and remain valid until cleanup.
            /// `registry_path` must be a non-null pointer to a valid native
            /// `UNICODE_STRING` descriptor, with its buffer valid throughout
            /// initialization. Pass native pointers, not framework wrapper
            /// addresses. Initialization and cleanup must be externally
            /// serialized; do not change context during an active lifecycle.
            /// Call `cleanup` before unloading the tracing code.
            pub unsafe fn init_driver(
                driver_object: *mut ::core::ffi::c_void,
                registry_path: *const ::core::ffi::c_void,
            ) {
                #ifr_init_driver
                unsafe { init_providers(); }
            }

            #app_init

            unsafe fn init_providers() {
                #(#init_calls)*
            }

            /// Cleans up all providers in reverse order, then IFR when enabled.
            ///
            /// # Safety
            ///
            /// All tracing callers must have stopped, and no initialization or
            /// other cleanup may run concurrently. The driver object supplied
            /// at initialization must still be valid.
            pub unsafe fn cleanup() {
                #(#cleanup_calls)*
                #ifr_cleanup
            }
        }
    }
}

/// Generates the `__wpp_ifr` module containing all IFR infrastructure:
///
/// - `WPP_GLOBAL_Control` / `WPP_RECORDER_INITIALIZED` (`#[no_mangle]` statics)
/// - Control block array and helper types
/// - `STATE: IFRState` — holds auto-log context for trace calls
/// - `init_driver(driver_object, registry_path)` — starts driver IFR
/// - `init()` — starts application IFR in user mode
/// - `cleanup()` — stops IFR tracing
fn generate_ifr_module(providers: &[ProviderDecl]) -> TokenStream {
    let app_init = if cfg!(feature = "user_mode") {
        quote! {
            /// Initializes application IFR without a driver context.
            ///
            /// # Safety
            ///
            /// Initialization and cleanup must be externally serialized.
            /// Do not change context during an active tracing lifecycle.
            #[doc(hidden)]
            pub unsafe fn init() {
                unsafe { init_raw(::core::ptr::null_mut(), ::core::ptr::null()); }
            }
        }
    } else {
        TokenStream::new()
    };
    let num_controls = providers.len();
    let num_controls_lit = proc_macro2::Literal::usize_unsuffixed(num_controls);

    // WPP_FLAG_LEN: max number of 32-bit flag words needed
    let wpp_flag_len: usize = providers
        .iter()
        .map(|p| (p.keywords.len() + 31) / 32)
        .max()
        .unwrap_or(0)
        .max(1);
    let wpp_flag_len_lit = proc_macro2::Literal::usize_unsuffixed(wpp_flag_len);

    // TMC codeview annotations for each provider
    // let codeview_annotations: Vec<TokenStream> = providers
    //     .iter()
    //     .map(|p| {
    //         let guid = &p.guid_str;
    //         let provider_name = p.name.to_string();
    //         let flags: Vec<String> = p.keywords.iter().map(|kw|
    // kw.name.to_string()).collect();         quote! {
    //             ::core::os::windows::codeview_annotation!("TMC:", #guid, #provider_name,
    // #(#flags),*);         }
    //     })
    //     .collect();

    // Link Next pointers between control blocks
    let link_statements: Vec<TokenStream> = (0..num_controls)
        .map(|idx| {
            let idx_lit = proc_macro2::Literal::usize_unsuffixed(idx);
            let next_expr = if idx + 1 < num_controls {
                let next_idx = proc_macro2::Literal::usize_unsuffixed(idx + 1);
                quote! {
                    __wpp_base.add(#next_idx)
                        .cast::<::wpp::ifr::WPP_TRACE_CONTROL_BLOCK>()
                        as *const ::wpp::ifr::WPP_TRACE_CONTROL_BLOCK
                }
            } else {
                quote! { ::core::ptr::null() }
            };
            quote! {
                {
                    let control = unsafe { &mut *__wpp_arr[#idx_lit].Control };
                    control.Next = unsafe { #next_expr };
                }
            }
        })
        .collect();

    // Set ControlGuid on each CB from the global WPP_CONTROL_GUIDS array
    let set_guid_statements: Vec<TokenStream> = (0..num_controls)
        .map(|idx| {
            let idx_lit = proc_macro2::Literal::usize_unsuffixed(idx);
            quote! {
                unsafe {
                    (*get_cb(#idx_lit)).ControlGuid =
                        &super::WPP_CONTROL_GUIDS[#idx_lit] as *const ::wpp::GUID;
                }
            }
        })
        .collect();

    quote! {
        /// IFR (In-Flight Recorder) module — manages control blocks, IFR
        /// lifecycle, and holds the shared `IFRState`.
        #[allow(non_snake_case)]
        pub mod __wpp_ifr {
            // IFR globals — must live in the consuming crate (not in wpp lib
            // crate) to avoid LTO bitcode errors with #[no_mangle] statics.
            #[unsafe(no_mangle)]
            static mut WPP_GLOBAL_Control: *mut ::wpp::ifr::WPP_PROJECT_CONTROL_BLOCK =
                ::core::ptr::null_mut();

            // ── Control block array types ────────────────────────────────────

            const WPP_FLAG_LEN: usize = #wpp_flag_len_lit;
            const CONTROLS_COUNT: usize = #num_controls_lit;

            #[repr(C)]
            union CbType {
                Control: ::core::mem::ManuallyDrop<::wpp::ifr::WPP_TRACE_CONTROL_BLOCK>,
                ReserveSpace: [u8;
                    ::core::mem::size_of::<::wpp::ifr::WPP_TRACE_CONTROL_BLOCK>()
                        + ::core::mem::size_of::<u32>() * (WPP_FLAG_LEN - 1)],
            }

            #[repr(transparent)]
            struct ControlBlockArray(
                ::core::cell::UnsafeCell<[CbType; CONTROLS_COUNT]>,
            );

            unsafe impl ::core::marker::Sync for ControlBlockArray {}

            impl ControlBlockArray {
                const fn new() -> Self {
                    const INIT: CbType = CbType {
                        Control: ::core::mem::ManuallyDrop::new(
                            ::wpp::ifr::WPP_TRACE_CONTROL_BLOCK::new(
                                #wpp_flag_len_lit as u8,
                            ),
                        ),
                    };
                    Self(::core::cell::UnsafeCell::new([INIT; CONTROLS_COUNT]))
                }
                fn as_mut_ptr(&self) -> *mut [CbType; CONTROLS_COUNT] {
                    self.0.get()
                }
            }

            static MAIN_CB: ControlBlockArray = ControlBlockArray::new();

            // ── IFR state ────────────────────────────────────────────────────

            /// Shared IFR state holding the auto-log context for trace calls.
            pub static STATE: ::wpp::ifr::IFRState = ::wpp::ifr::IFRState::new();

            // ── IFR functions ────────────────────────────────────────────────

            /// Returns a mutable pointer to the control block at `idx`.
            #[inline]
            fn get_cb(idx: usize) -> *mut ::wpp::ifr::WPP_TRACE_CONTROL_BLOCK {
                unsafe {
                    (*MAIN_CB.as_mut_ptr())
                        .as_mut_ptr()
                        .add(idx)
                        .cast::<::wpp::ifr::WPP_TRACE_CONTROL_BLOCK>()
                }
            }

            /// Initializes driver IFR using native context pointers.
            ///
            /// # Safety
            ///
            /// `driver_object` must be a non-null, valid native driver-object
            /// pointer for the selected mode and remain valid through cleanup.
            /// `registry_path` must be a non-null pointer to a valid native
            /// `UNICODE_STRING` descriptor, with its buffer valid throughout
            /// initialization. Pass native pointers, not framework wrapper
            /// addresses.
            /// Initialization and cleanup must be externally serialized.
            /// Do not change context during an active tracing lifecycle.
            #[doc(hidden)]
            pub unsafe fn init_driver(
                driver_object: *mut ::core::ffi::c_void,
                registry_path: *const ::core::ffi::c_void,
            ) {
                unsafe { init_raw(driver_object, registry_path); }
            }

            #app_init

            unsafe fn init_raw(
                driver_obj: *mut ::core::ffi::c_void,
                reg_path: *const ::core::ffi::c_void,
            ) {
                if STATE.init_state.compare_exchange(
                    ::wpp::ifr::UNINITIALIZED,
                    ::wpp::ifr::INITIALIZING,
                    ::core::sync::atomic::Ordering::Acquire,
                    ::core::sync::atomic::Ordering::Relaxed,
                ).is_err() {
                    return;
                }

                // Emit TMC codeview annotations
                // #(#codeview_annotations)*

                // Link control block Next pointers
                let __wpp_arr: &mut [CbType; CONTROLS_COUNT] =
                    unsafe { &mut *MAIN_CB.as_mut_ptr() };
                let __wpp_base: *mut CbType = __wpp_arr.as_mut_ptr();
                #(#link_statements)*

                // Set ControlGuid on each CB from the global GUID array
                #(#set_guid_statements)*

                // Start IFR auto-log recording
                let cb_ptr = unsafe {
                    (*MAIN_CB.as_mut_ptr())
                        .as_mut_ptr()
                        .cast::<::wpp::ifr::WPP_PROJECT_CONTROL_BLOCK>()
                };

                // TODO: Handle start error here
                unsafe {
                    ::wpp::ifr::start_ifr(
                        cb_ptr,
                        driver_obj,
                        reg_path,
                        &STATE,
                        &raw mut WPP_GLOBAL_Control,
                    );
                }

                STATE.init_state.store(
                    ::wpp::ifr::INITIALIZED,
                    ::core::sync::atomic::Ordering::Release,
                );
            }

            /// Stops IFR tracing and clears the auto-log context.
            ///
            /// # Safety
            ///
            /// All tracing callers must have stopped. Initialization and
            /// cleanup must be externally serialized, and the driver object
            /// supplied at initialization must still be valid.
            #[doc(hidden)]
            pub unsafe fn cleanup() {
                if STATE.init_state.compare_exchange(
                    ::wpp::ifr::INITIALIZED,
                    ::wpp::ifr::UNINITIALIZED,
                    ::core::sync::atomic::Ordering::Acquire,
                    ::core::sync::atomic::Ordering::Relaxed,
                ).is_err() {
                    return;
                }
                unsafe {
                    ::wpp::ifr::stop_ifr(
                        &STATE,
                        &raw mut WPP_GLOBAL_Control,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_module_initializes_and_cleans_up_in_order() {
        let parsed: ControlGuidsInput = syn::parse2(quote! {
            First "34746ee9-2982-4967-aac1-fece7a3dfc3c" { GENERAL }
            Second "c5a04b92-0ed8-45ec-9732-aa27fdc95e45" { OTHER }
        })
        .unwrap();
        let module: syn::ItemMod =
            syn::parse2(generate_lifecycle_module(&parsed.providers)).unwrap();
        assert_eq!(module.ident, "wpp_tracing");
        let functions: Vec<_> = module
            .content
            .unwrap()
            .1
            .into_iter()
            .map(|item| {
                let syn::Item::Fn(function) = item else {
                    panic!("expected a lifecycle function");
                };
                function
            })
            .collect();
        let find = |name: &str| functions.iter().find(|f| f.sig.ident == name).unwrap();
        let driver_init = find("init_driver");
        assert!(driver_init.sig.unsafety.is_some());
        let parameters = &driver_init.sig.inputs;
        assert_eq!(
            quote!(#parameters).to_string(),
            "driver_object : * mut :: core :: ffi :: c_void , registry_path : * const :: core :: ffi :: c_void ,"
        );
        let driver_body = &driver_init.block;
        let driver_body = quote!(#driver_body).to_string();
        let providers = &find("init_providers").block;
        let providers = quote!(#providers).to_string();
        let cleanup = find("cleanup");
        assert!(cleanup.sig.unsafety.is_some());
        let cleanup = &cleanup.block;
        let cleanup = quote!(#cleanup).to_string();
        assert!(
            providers.find("First :: init").unwrap() < providers.find("Second :: init").unwrap()
        );
        assert!(
            cleanup.find("Second :: clean_up").unwrap()
                < cleanup.find("First :: clean_up").unwrap()
        );
        assert_eq!(driver_body.contains("__wpp_ifr"), cfg!(feature = "ifr"));
        assert_eq!(cleanup.contains("__wpp_ifr"), cfg!(feature = "ifr"));
        if cfg!(feature = "ifr") {
            assert!(
                driver_body.find("__wpp_ifr :: init_driver").unwrap()
                    < driver_body.find("init_providers").unwrap()
            );
            assert!(
                cleanup.find("First :: clean_up").unwrap()
                    < cleanup.find("__wpp_ifr :: cleanup").unwrap()
            );
        }
        let app_init = functions.iter().find(|f| f.sig.ident == "init");
        assert_eq!(app_init.is_some(), cfg!(feature = "user_mode"));
        if let Some(app_init) = app_init {
            assert!(app_init.sig.inputs.is_empty());
            assert!(app_init.sig.unsafety.is_some());
            let body = &app_init.block;
            let body = quote!(#body).to_string();
            assert_eq!(body.contains("__wpp_ifr"), cfg!(feature = "ifr"));
            if cfg!(feature = "ifr") {
                assert!(
                    body.find("__wpp_ifr :: init").unwrap() < body.find("init_providers").unwrap()
                );
            }
        }
    }

    #[test]
    fn ifr_module_requires_ifr_feature() {
        let output = generate(quote! {
            App "34746ee9-2982-4967-aac1-fece7a3dfc3c" { GENERAL }
        })
        .unwrap();
        let file: syn::File = syn::parse2(output).unwrap();
        let has_ifr_module = file
            .items
            .iter()
            .any(|item| matches!(item, syn::Item::Mod(module) if module.ident == "__wpp_ifr"));
        assert_eq!(has_ifr_module, cfg!(feature = "ifr"));
        assert!(
            file.items
                .iter()
                .any(|item| { matches!(item, syn::Item::Mod(module) if module.ident == "App") })
        );
        assert!(file.items.iter().any(|item| {
            matches!(item, syn::Item::Mod(module) if module.ident == "wpp_tracing")
        }));
        assert!(!file.items.iter().any(|item| {
            matches!(item, syn::Item::Macro(item) if item.ident.as_ref().is_some_and(
                |name| name == "wpp_init_tracing" || name == "wpp_cleanup"
            ))
        }));
    }

    #[test]
    fn ifr_entry_points_share_raw_initialization() {
        let parsed: ControlGuidsInput = syn::parse2(quote! {
            App "34746ee9-2982-4967-aac1-fece7a3dfc3c" { GENERAL }
        })
        .unwrap();
        let module: syn::ItemMod = syn::parse2(generate_ifr_module(&parsed.providers)).unwrap();
        let functions: Vec<_> = module
            .content
            .unwrap()
            .1
            .into_iter()
            .filter_map(|item| match item {
                syn::Item::Fn(function) => Some(function),
                _ => None,
            })
            .collect();
        let driver_init = functions
            .iter()
            .find(|f| f.sig.ident == "init_driver")
            .unwrap();
        assert!(driver_init.sig.unsafety.is_some());
        let parameters = &driver_init.sig.inputs;
        assert_eq!(
            quote!(#parameters).to_string(),
            "driver_object : * mut :: core :: ffi :: c_void , registry_path : * const :: core :: ffi :: c_void ,"
        );
        let body = &driver_init.block;
        let body = quote!(#body).to_string();
        assert!(body.contains("init_raw (driver_object , registry_path)"));
        assert!(!body.contains("compare_exchange"));
        let app_init = functions.iter().find(|f| f.sig.ident == "init");
        assert_eq!(app_init.is_some(), cfg!(feature = "user_mode"));
        if let Some(app_init) = app_init {
            assert!(app_init.sig.inputs.is_empty());
            let body = &app_init.block;
            let body = quote!(#body).to_string();
            assert!(
                body.contains(
                    "init_raw (:: core :: ptr :: null_mut () , :: core :: ptr :: null ())"
                )
            );
        }
        let raw_init = functions
            .iter()
            .find(|f| f.sig.ident == "init_raw")
            .unwrap();
        assert!(matches!(raw_init.vis, syn::Visibility::Inherited));
        let body = &raw_init.block;
        assert!(quote!(#body).to_string().contains("compare_exchange"));
    }

    #[test]
    fn lifecycle_module_names_are_reserved() {
        for name in ["wpp_tracing", "__wpp_ifr"] {
            let name = Ident::new(name, proc_macro2::Span::call_site());
            let error = generate(quote! {
                #name "34746ee9-2982-4967-aac1-fece7a3dfc3c" { GENERAL }
            })
            .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("reserved for WPP lifecycle support")
            );
        }
    }

    #[test]
    fn guid_parsing() {
        let gp = parse_guid_parts("84bdb2e9-829e-41b3-b891-02f454bc2bd7").unwrap();
        assert_eq!(gp.d1, 0x84BDB2E9);
        assert_eq!(gp.d2, 0x829E);
        assert_eq!(gp.d3, 0x41B3);
    }

    #[test]
    fn quoted_guid_accepts_segments_rust_cannot_lex() {
        let input = quote! {
            App "34746ee9-2982-4967-aac1-fece7a3dfc3c" { GENERAL }
        };
        let provider: ProviderDecl = syn::parse2(input).unwrap();
        assert_eq!(provider.guid_str, "34746ee9-2982-4967-aac1-fece7a3dfc3c");
        assert_eq!(provider.guid_parts.d1, 0x34746EE9);
    }

    #[test]
    fn unquoted_guid_is_rejected() {
        let input = quote! {
            App cb94defb-592a-4509-8f2e-54f204929669 { GENERAL }
        };
        let error = syn::parse2::<ProviderDecl>(input).err().unwrap();
        assert!(error.to_string().contains("expected a quoted control GUID"));
    }

    #[test]
    fn quoted_guid_with_whitespace_is_rejected() {
        let input = quote! {
            App " 34746ee9-2982-4967-aac1-fece7a3dfc3c" { GENERAL }
        };
        let error = syn::parse2::<ProviderDecl>(input).err().unwrap();
        assert!(error.to_string().contains("GUID segment 0 has length 9"));
    }

    #[test]
    fn quoted_guid_is_validated() {
        let input = quote! {
            App "34746ee9-2982-4967-aac1" { GENERAL }
        };
        let error = syn::parse2::<ProviderDecl>(input).err().unwrap();
        assert!(error.to_string().contains("5 dash-separated segments"));
    }
}
