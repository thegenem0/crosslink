use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::Error as SynError;
use syn::parse_macro_input;

mod model;
use model::*;

extern crate proc_macro;

#[proc_macro]
#[allow(unused_variables)]
#[allow(non_snake_case)]
pub fn define_crosslink(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as DefineCommsLinkInput);
    // let router_expr = &parsed.router_arg.expr;
    let link_id_base = &parsed.link_id_arg.name.value();

    let ep1_handle_name = &parsed.ep1_def.handle_name;
    let ep1_sends_type = &parsed.ep1_def.messages.sends_ty;
    let ep1_receives_type = &parsed.ep1_def.messages.receives_ty;

    let ep2_handle_name = &parsed.ep2_def.handle_name;
    let ep2_sends_type = &parsed.ep2_def.messages.sends_ty;
    let ep2_receives_type = &parsed.ep2_def.messages.receives_ty;

    // Assert opposing directions are the same type
    // Will be properly validated during compilation
    assert_eq!(ep1_sends_type, ep2_receives_type);
    assert_eq!(ep1_receives_type, ep2_sends_type);

    let buffer_usize_val = match parsed.buffer_arg.value.base10_parse::<usize>() {
        Ok(val) => val,
        Err(e) => {
            return SynError::new_spanned(
                &parsed.buffer_arg.value,
                format!("Failed to parse usize from buffer_size value: {}", e),
            )
            .to_compile_error()
            .into();
        }
    };

    let sender_marker_ep1 = format_ident!("{}Send", ep1_handle_name);
    let receiver_marker_ep1 = format_ident!("{}Recv", ep1_handle_name);
    let sender_marker_ep2 = format_ident!("{}Send", ep2_handle_name);
    let receiver_marker_ep2 = format_ident!("{}Recv", ep2_handle_name);

    let tx1 = format_ident!(
        "__tx_{}_{}",
        link_id_base.to_lowercase(),
        ep1_handle_name.to_string().to_lowercase()
    ); // For ep1 sending

    let rx1 = format_ident!(
        "__rx_{}_{}",
        link_id_base.to_lowercase(),
        ep1_handle_name.to_string().to_lowercase()
    ); // For ep1 receiving

    let tx2 = format_ident!(
        "__tx_{}_{}",
        link_id_base.to_lowercase(),
        ep2_handle_name.to_string().to_lowercase()
    ); // For ep2 sending

    let rx2 = format_ident!(
        "__rx_{}_{}",
        link_id_base.to_lowercase(),
        ep2_handle_name.to_string().to_lowercase()
    ); // For ep2 receiving

    let mod_name = format_ident!("{}", link_id_base.to_snake_case());
    let setup_fn_name = format_ident!("setup_{}", mod_name);

    let crosslink_crate_path = quote!(::crosslink);
    let router_path = quote!(#crosslink_crate_path::Router);

    let definitions_q = quote! {
        pub mod #mod_name {
            use super::*;

            pub mod marker {
                use super::*;

                #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
                #[allow(non_snake_case, dead_code)]
                pub struct #sender_marker_ep1;

                #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
                #[allow(non_snake_case, dead_code)]
                pub struct #receiver_marker_ep1;

                #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
                #[allow(non_snake_case, dead_code)]
                pub struct #sender_marker_ep2;

                #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
                #[allow(non_snake_case, dead_code)]
                pub struct #receiver_marker_ep2;
            }


            #[derive(Debug, Clone, Copy)]
            #[allow(non_snake_case, dead_code)]
            pub struct #ep1_handle_name;

            #[derive(Debug, Clone, Copy)]
            #[allow(non_snake_case, dead_code)]
            pub struct #ep2_handle_name;

            #[allow(dead_code)]
            pub fn #setup_fn_name(
                router: &mut #router_path,
                buffer_size_override: Option<usize>,
            ) -> (
                #ep1_handle_name,
                #ep2_handle_name,
            ) {
                let buffer_val = buffer_size_override.unwrap_or(#buffer_usize_val);

                // Channel for ep1_sends_ty (sent by ep1, received by ep2)
                let (#tx1, #rx2) = ::tokio::sync::mpsc::channel::<#ep1_sends_type>(buffer_val);
                // Channel for ep2_sends_ty (sent by ep2, received by ep1)
                let (#tx2, #rx1) = ::tokio::sync::mpsc::channel::<#ep2_sends_type>(buffer_val);

                router.__internal_register_sender::<marker::#sender_marker_ep1, #ep1_sends_type>(#tx1)
                    .unwrap_or_else(|e| panic!("Macro Setup Error ({}): {}", stringify!(#sender_marker_ep1), e));

                router.__internal_register_receiver::<marker::#receiver_marker_ep1, #ep2_sends_type>(#rx1)
                    .unwrap_or_else(|e| panic!("Macro Setup Error ({}): {}", stringify!(#receiver_marker_ep1), e));

                router.__internal_register_sender::<marker::#sender_marker_ep2, #ep2_sends_type>(#tx2)
                    .unwrap_or_else(|e| panic!("Macro Setup Error ({}): {}", stringify!(#sender_marker_ep2), e));

                router.__internal_register_receiver::<marker::#receiver_marker_ep2, #ep1_sends_type>(#rx2)
                    .unwrap_or_else(|e| panic!("Macro Setup Error ({}): {}", stringify!(#receiver_marker_ep2), e));

                (#ep1_handle_name, #ep2_handle_name)
            }
        }
    };

    definitions_q.into()
}
