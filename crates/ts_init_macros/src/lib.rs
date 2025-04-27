use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};

#[proc_macro]
pub fn env_filter_directive(input: TokenStream) -> TokenStream {
    let level = parse_macro_input!(input as LitStr).value();
    let pkg = std::env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME must be set by Cargo");
    let bin = std::env::var("CARGO_BIN_NAME").unwrap_or_else(|_| pkg.clone());

    let directive = if pkg == bin {
        format!("{}={}", pkg, level)
    } else {
        format!("{pkg}={lvl},{bin}={lvl}", pkg = pkg, lvl = level, bin = bin)
    };

    let lit = LitStr::new(&directive, proc_macro2::Span::call_site());
    TokenStream::from(quote!( #lit ))
}
