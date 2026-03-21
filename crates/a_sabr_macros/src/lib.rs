use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(DefaultNodeRx)]
pub fn derive_default_node_rx(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        impl NodeRx for #name {
            fn dry_run_rx(&self, _start: Date, _end: Date, _bundle: &Bundle) -> bool {
                true
            }

            fn schedule_rx(&mut self, _start: Date, _end: Date, _bundle: &Bundle) -> bool {
                true
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(DefaultNodeTx)]
pub fn derive_default_node_tx(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        impl NodeTx for #name {
            fn dry_run_tx(&self, _waiting_since: Date, _start: Date, _end: Date, _bundle: &Bundle) -> bool {
                true
            }

            fn schedule_tx(&mut self, _waiting_since: Date, _start: Date, _end: Date, _bundle: &Bundle) -> bool {
                true
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(DefaultNodeManager)]
pub fn derive_default_node_manager(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        impl NodeManager for #name {
            fn dry_run_process(&self, at_time: Date, _bundle: &mut Bundle) -> Date {
                at_time
            }

            fn schedule_process(&self, at_time: Date, _bundle: &mut Bundle) -> Date {
                at_time
            }
        }
    };

    TokenStream::from(expanded)
}
