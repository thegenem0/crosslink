use syn::{
    Error as SynError, Ident, LitInt, LitStr, Result as SynResult, Token, Type, braced,
    parse::{Parse, ParseStream},
    token,
};

/// LinkIdArg:
/// `link_id: "ExampleLinkName"`
pub struct LinkIdArg {
    pub _kw: Ident,
    pub _col: Token![:],
    pub name: LitStr,
    pub _com: Token![,],
}

impl Parse for LinkIdArg {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let _kw: Ident = input.parse()?;
        if _kw != "link_id" {
            return Err(SynError::new_spanned(&_kw, "Expected 'link_id' keyword"));
        }

        Ok(LinkIdArg {
            _kw,
            _col: input.parse()?,
            name: input.parse()?,
            _com: input.parse()?,
        })
    }
}

pub struct EndpointMessages {
    pub _sends_kw: Ident,
    pub _s_col: Token![:],
    pub sends_ty: Type,
    pub _s_com: Token![,],
    pub _rec_kw: Ident,
    pub _r_col: Token![:],
    pub receives_ty: Type,
    pub _r_com: Option<Token![,]>,
}

impl Parse for EndpointMessages {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let _sends_kw = input.parse()?;
        if _sends_kw != "sends" {
            return Err(SynError::new_spanned(_sends_kw, "Expected 'sends'"));
        }

        let _s_col = input.parse()?;
        let sends_ty = input.parse()?;
        let _s_com = input.parse()?;
        let _rec_kw = input.parse()?;
        if _rec_kw != "receives" {
            return Err(SynError::new_spanned(_rec_kw, "Expected 'receives'"));
        }

        let _r_col = input.parse()?;
        let receives_ty = input.parse()?;
        let _r_com = input.parse().ok();

        Ok(Self {
            _sends_kw,
            _s_col,
            sends_ty,
            _s_com,
            _rec_kw,
            _r_col,
            receives_ty,
            _r_com,
        })
    }
}

pub struct EndpointDef {
    pub handle_name: Ident,
    pub _brace: token::Brace,
    pub messages: EndpointMessages,
    pub _com: Token![,],
}

impl Parse for EndpointDef {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let handle_name = input.parse()?;

        let content;
        let _brace = braced!(content in input);
        let messages = content.parse()?;

        if !content.is_empty() {
            return Err(SynError::new(
                content.span(),
                "Unexpected tokens in endpoint def",
            ));
        }

        Ok(Self {
            handle_name,
            _brace,
            messages,
            _com: input.parse()?,
        })
    }
}

pub struct BufferArg {
    pub _kw: Ident,
    pub _col: Token![:],
    pub value: LitInt,
    pub _com: Option<Token![,]>,
}

impl Parse for BufferArg {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let _kw = input.parse()?;
        if _kw != "buffer_size" {
            return Err(SynError::new_spanned(_kw, "Expected 'buffer_size'"));
        }

        Ok(Self {
            _kw,
            _col: input.parse()?,
            value: input.parse()?,
            _com: input.parse().ok(),
        })
    }
}

pub struct DefineCommsLinkInput {
    pub link_id_arg: LinkIdArg,
    pub ep1_def: EndpointDef,
    pub ep2_def: EndpointDef,
    pub buffer_arg: BufferArg,
}

impl Parse for DefineCommsLinkInput {
    fn parse(input: ParseStream) -> SynResult<Self> {
        Ok(Self {
            link_id_arg: input.parse()?,
            ep1_def: input.parse()?,
            ep2_def: input.parse()?,
            buffer_arg: input.parse()?,
        })
    }
}
