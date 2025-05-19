use std::collections::HashSet;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    braced, parenthesized, parse::{Parse, ParseStream}, parse_macro_input, token, Error as SynError, Ident, LitInt, Result as SynResult, Token, Type
};

extern crate proc_macro;

// --- Meta Arguments ---
// enum LinkKeys;
struct MetaEnum {
    _enum_kw: Token![enum],
    name: Ident,
    _semicolon: Token![;],
}

impl Parse for MetaEnum {
    fn parse(input: ParseStream) -> SynResult<Self> {
        Ok(MetaEnum {
            _enum_kw: input.parse()?,
            name: input.parse()?,
            _semicolon: input.parse()?,
        })
    }
}

struct MetaStruct {
    _struct_kw: Token![struct],
    name: Ident,
    _semicolon: Token![;],
}

impl Parse for MetaStruct {
    fn parse(input: ParseStream) -> SynResult<Self> {
        Ok(MetaStruct {
            _struct_kw: input.parse()?,
            name: input.parse()?,
            _semicolon: input.parse()?,
        })
    }
}

// --- Link Field Arguments ---
// name: CoordHandle, (comma is optional if last)
struct NameArg {
    _name_kw: Ident,
    _colon: Token![:],
    value: Ident,
    _comma: Option<Token![,]>,
}

impl Parse for NameArg {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let name_kw = input.parse()?;
        if name_kw != "name" {
            return Err(SynError::new_spanned(&name_kw, "Expected 'name' keyword"));
        }
        Ok(NameArg {
            _name_kw: name_kw,
            _colon: input.parse()?,
            value: input.parse()?,
            _comma: input.parse().ok(),
        })
    }
}

// sends: MyMessage, (comma is optional if last)
// receives: MyMessage, (comma is optional if last)
struct MsgTypeArg {
    kw: Ident, // "sends" or "receives"
    _colon: Token![:],
    ty: Type,
    _comma: Option<Token![,]>,
}

impl Parse for MsgTypeArg {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let kw: syn::Ident = input.parse()?;
        if kw != "sends" && kw != "receives" {
            return Err(SynError::new_spanned(
                &kw,
                "Expected 'sends' or 'receives' keyword",
            ));
        }

        Ok(MsgTypeArg {
            kw,
            _colon: input.parse()?,
            ty: input.parse()?,
            _comma: input.parse().ok(),
        })
    }
}

// buffer: 32, (comma is optional if last)
struct BufferArg {
    _buffer_kw: Ident,
    _colon: Token![:],
    value: LitInt,
    _comma: Option<Token![,]>,
}
impl Parse for BufferArg {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let buffer_kw: Ident = input.parse()?;
        if buffer_kw != "buffer" {
            return Err(SynError::new_spanned(
                &buffer_kw,
                "Expected 'buffer' keyword",
            ));
        }

        Ok(BufferArg {
            _buffer_kw: buffer_kw,
            _colon: input.parse()?,
            value: input.parse()?,
            _comma: input.parse().ok(),
        })
    }
}

// --- Endpoint Definitions ---
// ep1 ( name: Foo, sends: Msg1, receives: Msg2 ),
struct BiDiEndpointInput {
    id_kw: Ident, // "ep1" or "ep2"
    _paren_token: token::Paren,
    name_arg: NameArg,
    // For ep1, these are specified. For ep2, they are None and inferred later.
    sends_arg: Option<MsgTypeArg>,
    receives_arg: Option<MsgTypeArg>,
}

impl Parse for BiDiEndpointInput {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let id_kw: Ident = input.parse()?;
        if id_kw != "endpoint1" && id_kw != "endpoint2" {
            return Err(SynError::new_spanned(
                &id_kw,
                "Expected 'endpoint1' or 'endpoint2'",
            ));
        }

        let content;
        let paren_token = parenthesized!(content in input);

        let name_arg = content.parse()?;
        let mut sends_arg = None;
        let mut receives_arg = None;

        if id_kw == "endpoint1" {
            // endpoint1 must define both sends and receives, in this specific order for simplicity
            if !content.peek(Ident) {
                return Err(SynError::new(
                    content.span(),
                    "endpoint1 definition: expected 'sends: MessageType' as the first message type argument.",
                    ));
            }
            let arg1: MsgTypeArg = content.parse()?;
            if arg1.kw == "sends" {
                sends_arg = Some(arg1);
            } else {
                return Err(SynError::new_spanned(
                    arg1.kw,
                    "endpoint1 definition: expected 'sends: MessageType' as the first message type argument.",
                ));
            }

            if !content.peek(Ident) {
                return Err(SynError::new(
                    content.span(),
                    "endpoint1 definition: expected 'receives: MessageType' as the second message type argument."
                ));
            }

            let arg2: MsgTypeArg = content.parse()?;
            if arg2.kw == "receives" {
                receives_arg = Some(arg2);
            } else {
                return Err(SynError::new_spanned(
                    arg2.kw,
                    "endpoint1 definition: expected 'receives: MessageType' as the second message type argument.",
                ));
            }

            // Ensure content is empty after parsing name, sends, and receives for ep1
            if !content.is_empty() {
                return Err(SynError::new(
                    content.span(),
                    "endpoint1 definition: unexpected arguments after 'name', 'sends', and 'receives'.",
                ));
            }
        } else { // id_kw == "endpoint2"
            // Ensure content is empty after parsing name_arg for ep2
            if !content.is_empty() {
                return Err(SynError::new(
                    content.span(),
                    "endpoint2 definition should only contain 'name: HandleName,'. Message types are inferred.",
                ));
            }
        }

        Ok(BiDiEndpointInput {
            id_kw,
            _paren_token: paren_token,
            name_arg,
            sends_arg,
            receives_arg,
        })
    }}

// sender ( name: SenderHandle ),
struct SenderEpInput {
    _sender_kw: Ident,
    _paren_token: token::Paren,
    name_arg: NameArg,
}
impl Parse for SenderEpInput {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let sender_kw: Ident = input.parse()?;
        if sender_kw != "sender" {
            return Err(SynError::new_spanned(
                sender_kw,
                "Expected 'sender' keyword",
            ));
        }
        let content;
        let paren_token = parenthesized!(content in input);
        let name_arg = content.parse()?;
        if !content.is_empty() {
            return Err(SynError::new(
                content.span(),
                "Sender definition should only contain 'name' argument.",
            ));
        }
        Ok(SenderEpInput {
            _sender_kw: sender_kw,
            _paren_token: paren_token,
            name_arg,
        })
    }
}

// receiver ( name: ReceiverHandle, receives: MyEvent ),
struct ReceiverEpInput {
    _receiver_kw: Ident,
    _paren_token: token::Paren,
    name_arg: NameArg,
    receives_arg: MsgTypeArg,
}
impl Parse for ReceiverEpInput {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let receiver_kw: Ident = input.parse()?;
        if receiver_kw != "receiver" {
            return Err(syn::Error::new_spanned(
                receiver_kw,
                "Expected 'receiver' keyword",
            ));
        }
        let content;
        let paren_token = parenthesized!(content in input);
        let name_arg = content.parse()?;
        let receives_arg: MsgTypeArg = content.parse()?;
        if receives_arg.kw != "receives" {
            return Err(SynError::new_spanned(
                &receives_arg.kw,
                "Receiver expects 'receives' keyword for message type.",
            ));
        }
        if !content.is_empty() {
            return Err(SynError::new(
                content.span(),
                "Receiver definition has unexpected arguments after 'receives'.",
            ));
        }
        Ok(ReceiverEpInput {
            _receiver_kw: receiver_kw,
            _paren_token: paren_token,
            name_arg,
            receives_arg,
        })
    }
}

// LinkName: bi_directional ( ep1(...), ep2(...), buffer: N ),
struct BiDirectionalLinkInput {
    endpoint1_input: BiDiEndpointInput,
    _comma1: Token![,],
    endpoint2_input: BiDiEndpointInput,
    _comma2: Token![,],
    buffer_arg: BufferArg,
}
impl Parse for BiDirectionalLinkInput {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let ep1_input: BiDiEndpointInput = input.parse()?;
        if ep1_input.id_kw != "endpoint1" {
            return Err(SynError::new_spanned(
                &ep1_input.id_kw,
                "Expected 'endpoint1(...)' definition first in bi_directional link.",
            ));
        }

        if ep1_input.sends_arg.is_none() {
            return Err(SynError::new_spanned(
                &ep1_input.id_kw,
                "Expected 'sends: MessageType' argument in endpoint1 definition.",
            ));
        }

        if ep1_input.receives_arg.is_none() {
            return Err(SynError::new_spanned(
                &ep1_input.id_kw,
                "Expected 'receives: MessageType' argument in endpoint1 definition.",
            ));
        }

        let _comma1: Token![,] = input.parse()?;
        let ep2_input: BiDiEndpointInput = input.parse()?;
        if ep2_input.id_kw != "endpoint2" {
            return Err(SynError::new_spanned(
                &ep2_input.id_kw,
                "Expected 'endpoint2(...)' definition second in bi_directional link.",
            ));
        }

        if ep2_input.sends_arg.is_some() || ep2_input.receives_arg.is_some() {
            return Err(SynError::new_spanned(&ep2_input.id_kw, "endpoint2 should not define sends/receives types; they are inferred."));
        }

        let _comma2: Token![,] = input.parse()?;
        let buffer_arg: BufferArg = input.parse()?;
        if buffer_arg._comma.is_none() && !input.is_empty() {
            // If no comma after buffer, stream should be empty
            return Err(syn::Error::new(
                input.span(),
                "Unexpected tokens after buffer argument in bi_directional link definition.
                Ensure a comma separates arguments or no trailing tokens if buffer is last.",
            ));
        }

        Ok(BiDirectionalLinkInput {
            endpoint1_input: ep1_input,
            _comma1,
            endpoint2_input: ep2_input,
            _comma2,
            buffer_arg,
        })
    }
}

// --- Main Link Definition and Block ---
// LinkName: type ( ... ),
struct LinkDefinition {
    link_id_variant: Ident,
    _colon: Token![:],
    link_type_kw: Ident,
    _paren_token: token::Paren,
    // Parsed content based on link_type_kw
    bi_di_content: Option<BiDirectionalLinkInput>,
    //    uni_dir_content: Option<UnidirectionalLinkInput>,
}
impl Parse for LinkDefinition {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let link_id_variant: Ident = input.parse()?;
        let _colon: Token![:] = input.parse()?;
        let link_type_kw: Ident = input.parse()?;

        let content_stream;
        let paren_token = parenthesized!(content_stream in input);

        let mut bi_di_content = None;
        // let mut uni_dir_content = None;

        if link_type_kw == "bi_directional" {
            bi_di_content = Some(content_stream.parse()?);
        } else if link_type_kw == "unidirectional" {
            todo!();
            // uni_dir_content = Some(content_stream.parse()?);
        } else {
            return Err(SynError::new_spanned(
                link_type_kw,
                "Link type must be 'bi_directional' or 'unidirectional'",
            ));
        }

        // After parsing specific content, the content_stream should be empty.
        // The specific parsers (BiDirectionalLinkInput, etc.) should consume all their tokens.
        if !content_stream.is_empty() {
            return Err(SynError::new(
                content_stream.span(),
                "Unexpected tokens remaining inside link definition parentheses.",
            ));
        }

        Ok(LinkDefinition {
            link_id_variant,
            _colon,
            link_type_kw,
            _paren_token: paren_token,
            bi_di_content,
            // uni_dir_content,
        })
    }
}

// links { LinkName1: type(...), LinkName2: type(...), }
struct LinksBlock {
    _links_kw: Ident,
    _brace_token: token::Brace,
    link_definitions: syn::punctuated::Punctuated<LinkDefinition, Token![,]>,
}

impl Parse for LinksBlock {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let links_kw: Ident = input.parse()?;
        if links_kw != "links" {
            return Err(SynError::new_spanned(links_kw, "Expected 'links' keyword"));
        }

        let content;
        let brace_token = braced!(content in input);
        let link_definitions = content.parse_terminated(LinkDefinition::parse, Token![,])?;

        Ok(LinksBlock {
            _links_kw: links_kw,
            _brace_token: brace_token,
            link_definitions,
        })
    }
}

// --- Top Level Macro Input ---
struct DefineLinksInput {
    meta_enum: MetaEnum,
    meta_struct: MetaStruct,
    links_block: LinksBlock,
}

impl Parse for DefineLinksInput {
    fn parse(input: ParseStream) -> SynResult<Self> {
        Ok(DefineLinksInput {
            meta_enum: input.parse()?,
            meta_struct: input.parse()?,
            links_block: input.parse()?,
        })
    }
}

#[proc_macro]
pub fn define_links(input: TokenStream) -> TokenStream {
    let parsed_input = parse_macro_input!(input as DefineLinksInput);

    let link_enum_name = parsed_input.meta_enum.name;
    let handles_struct_name = parsed_input.meta_struct.name;

    let enum_variants = parsed_input
        .links_block
        .link_definitions
        .iter()
        .map(|link_def| &link_def.link_id_variant);

    let link_enum_def = quote! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ::strum_macros::AsRefStr)]
         #[strum(serialize_all = "PascalCase")]
         pub enum #link_enum_name {
             #( #enum_variants ),*
         }
     };

    let mut handle_struct_defs = Vec::new();
    let mut app_handles_fields = Vec::new();
    let mut router_setup_calls = Vec::new();
    let mut app_handles_instance_fields_assignments = Vec::new();
    let mut defined_handle_struct_names = HashSet::new();

    for link_def in parsed_input.links_block.link_definitions {
        let link_id_variant = &link_def.link_id_variant;
        let link_id_variant_str_lower = link_id_variant.to_string().to_lowercase();

        if let Some(bi_di_content) = link_def.bi_di_content {
            let ep1_data = &bi_di_content.endpoint1_input;
            let ep2_data = &bi_di_content.endpoint2_input;

            let ep1_handle_name = &ep1_data.name_arg.value;
            let ep1_sends_msg_type = ep1_data.sends_arg.as_ref().unwrap().ty.clone();
            let ep1_receives_msg_type = ep1_data.receives_arg.as_ref().unwrap().ty.clone();

            let ep2_handle_name = &ep2_data.name_arg.value;
            let ep2_sends_msg_type = ep2_data.sends_arg.as_ref().unwrap().ty.clone();
            let ep2_receives_msg_type = ep2_data.receives_arg.as_ref().unwrap().ty.clone();

            let buffer = &bi_di_content.buffer_arg.value;

            if defined_handle_struct_names.insert(ep1_handle_name.to_string()) {
                handle_struct_defs.push(quote! {
                    #[derive(Debug)]
                    pub struct #ep1_handle_name {
                        pub receiver: ::tokio::sync::mpsc::Receiver<#ep2_sends_msg_type>,
                        pub link_id: #link_enum_name,
                    }
                    impl #ep1_handle_name {
                        pub async fn recv(&mut self) -> Option<#ep2_sends_msg_type> {
                            self.receiver.recv().await
                        }
                        
                        pub async fn send_msg(&self, router: &::crosslink::Router, msg: #ep1_sends_msg_type) -> Result<(), ::crosslink::CommsError> {
                            router.send(#link_enum_name::#link_id_variant, msg).await
                        }
                    }
                });
            }

            if defined_handle_struct_names.insert(ep2_handle_name.to_string()) {
                handle_struct_defs.push(quote! {
                    #[derive(Debug)]
                    pub struct #ep2_handle_name {
                        pub receiver: ::tokio::sync::mpsc::Receiver<#ep1_receives_msg_type>,
                        pub link_id: #link_enum_name,
                    }

                    impl #ep2_handle_name {
                        pub async fn recv(&mut self) -> Option<#ep1_receives_msg_type> {
                            self.receiver.recv().await
                        }
                        
                        pub async fn send_msg(&self, router: &::crosslink::Router, msg: #ep2_sends_msg_type) -> Result<(), ::crosslink::CommsError> {
                            router.send(#link_enum_name::#link_id_variant, msg).await
                        }
                    }
                });
            }

            let field1_name = format_ident!("{}_{}", link_id_variant_str_lower, ep1_handle_name.to_string().to_lowercase());
            let field2_name = format_ident!("{}_{}", link_id_variant_str_lower, ep2_handle_name.to_string().to_lowercase());
            app_handles_fields.push(quote!( pub #field1_name: #ep1_handle_name ));
            app_handles_fields.push(quote!( pub #field2_name: #ep2_handle_name ));

            let tx_ep1_to_ep2 = format_ident!("tx_{}_ep1_to_ep2", link_id_variant_str_lower);
            let rx_ep2_from_ep1 = format_ident!("rx_{}_ep2_from_ep1", link_id_variant_str_lower);
            let tx_ep2_to_ep1 = format_ident!("tx_{}_ep2_to_ep1", link_id_variant_str_lower);
            let rx_ep1_from_ep2 = format_ident!("rx_{}_ep1_from_ep2", link_id_variant_str_lower);
            let var_ep1_handle = format_ident!("handle_{}", field1_name);
            let var_ep2_handle = format_ident!("handle_{}", field2_name);

            router_setup_calls.push(quote! {
                let (#tx_ep1_to_ep2, #rx_ep2_from_ep1) = ::tokio::sync::mpsc::channel::<#ep1_sends_msg_type>(#buffer.base10_parse::<usize>().unwrap());
                let (#tx_ep2_to_ep1, #rx_ep1_from_ep2) = ::tokio::sync::mpsc::channel::<#ep2_sends_msg_type>(#buffer.base10_parse::<usize>().unwrap());

                router.__internal_register_pathway_and_type_mapping::<#ep1_sends_msg_type>(
                    #link_enum_name::#link_id_variant.as_ref(),
                    stringify!(#ep1_handle_name), stringify!(#ep2_handle_name), #tx_ep1_to_ep2
                ).unwrap_or_else(|e| panic!("FATAL (Link: {}): Failed to register pathway {} -> {}: {:?}", stringify!(#link_id_variant), stringify!(#ep1_handle_name), stringify!(#ep2_handle_name), e));

                router.__internal_register_pathway_and_type_mapping::<#ep2_sends_msg_type>(
                    #link_enum_name::#link_id_variant.as_ref(),
                    stringify!(#ep2_handle_name), stringify!(#ep1_handle_name), #tx_ep2_to_ep1
                ).unwrap_or_else(|e| panic!("FATAL (Link: {}): Failed to register pathway {} -> {}: {:?}", stringify!(#link_id_variant), stringify!(#ep2_handle_name), stringify!(#ep1_handle_name), e));

                let #var_ep1_handle = #ep1_handle_name { receiver: #rx_ep1_from_ep2 , link_id: #link_enum_name::#link_id_variant };
                let #var_ep2_handle = #ep2_handle_name { receiver: #rx_ep2_from_ep1 , link_id: #link_enum_name::#link_id_variant };
            });

            app_handles_instance_fields_assignments.push(quote!( #field1_name: #var_ep1_handle ));
            app_handles_instance_fields_assignments.push(quote!( #field2_name: #var_ep2_handle ));
        } 
    }

    let handles_struct_def = quote! {
        #[derive(Debug)]
        pub struct #handles_struct_name {
            #( #app_handles_fields ),*
        }
    };

    let main_setup_block = quote! {
        {
            let mut router = ::crosslink::Router::new();

            #( #router_setup_calls )*

            let handles = #handles_struct_name {
                #( #app_handles_instance_fields_assignments ),*
            };
            (router, handles)
        }
    };

    let final_output = quote! {
        #link_enum_def
        #( #handle_struct_defs )*
        #handles_struct_def
        #main_setup_block
    };

    final_output.into()
}

