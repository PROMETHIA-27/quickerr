#![deny(missing_docs, rustdoc::all)]
#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use syn::__private::quote::quote;
use syn::__private::{ToTokens, TokenStream2};
use syn::parse::discouraged::Speculative;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::{self, Colon, Comma};
use syn::{bracketed, Attribute, Generics, Ident, LitStr, Result, Type, Visibility};

/// This macro allows quickly defining errors in the format that this crate produces.
///
/// It has 5 major forms:
/// - Unit struct:
/// ```
/// # use quickerr::error;
/// error! {
///     MyUnitError
///     "it's a unit error"
/// }
/// ```
/// - Record struct:
/// ```
/// # use quickerr::error;
/// # #[derive(Debug)]
/// # struct Type;
/// # #[derive(Debug)]
/// # struct Type2;
/// error! {
///     MyStructError
///     "it's a struct! Field 2 is {field2:?}"
///     field: Type,
///     field2: Type2,
/// }
/// ```
/// - Enum:
/// ```
/// # use quickerr::error;
/// # error! { SourceError1 "" }
/// # error! { MyUnitError "" }
/// # error! { MyStructError "" }
/// error! {
///     MyEnumError
///     "it's a whole enum"
///     SourceError1,
///     MyUnitError,
///     MyStructError,
/// }
/// ```
/// - Transparent enum:
/// ```
/// # use quickerr::error;
/// # error! { MyEnumError "uh oh" }
/// # error! { REALLY_LOUD_ERROR "uh oh" }
/// error! {
///     QuietAsAMouse
///     MyEnumError,
///     REALLY_LOUD_ERROR,
/// }
/// ```
/// - Array:
/// ```
/// # use quickerr::error;
/// # error! { SomeError "" }
/// error! {
///     ManyProblems
///     "encountered many problems"
///     [SomeError]
/// }
/// ```
///
/// Each form implements `Debug`, `Error`, and `From` as appropriate. The enum forms implement
/// [`std::error::Error::source()`] for each of their variants, and each variant must be the name
/// of an existing error type. The struct form exposes the fields for use in the error message.
/// The transparent enum form does not append a message, and simply passes the source along
/// directly. All forms are `#[non_exhaustive]` and all fields are public. They can be made public
/// by adding `pub` to the name like `pub MyError`.
///
/// Additional attributes can be added before the name to add them to the error type,
/// like so:
/// ```
/// # use quickerr::error;
/// error! {
///     #[derive(PartialEq, Eq)]
///     AttrsError
///     "has attributes!"
///     /// a number for something
///     num: i32
/// }
/// ```
///
/// Attributes can be added to fields and variants of struct/enum/array errors,
#[proc_macro]
pub fn error(tokens: TokenStream) -> TokenStream {
    match error_impl(tokens.into()) {
        Ok(toks) => toks.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn error_impl(tokens: TokenStream2) -> Result<TokenStream2> {
    let Error {
        attrs,
        vis,
        name,
        generics,
        msg,
        contents,
    } = syn::parse2(tokens)?;

    let (impl_gen, ty_gen, where_gen) = generics.split_for_impl();

    let write_msg = match &msg {
        Some(msg) => quote! {
            f.write_str(#msg)
        },
        None => {
            let sources = match &contents {
                ErrorContents::Enum { sources } => sources
                    .iter()
                    .map(|source| &source.ident)
                    .collect::<Vec<_>>(),
                _ => unreachable!(),
            };
            quote! {
                match self {
                    #(
                        Self::#sources(err) => ::std::fmt::Display::fmt(err, f),
                    )*
                }
            }
        }
    };

    Ok(match contents {
        ErrorContents::Unit => quote! {
            #(#attrs)*
            #[derive(Debug)]
            #[non_exhaustive]
            #vis struct #name #generics;

            impl #impl_gen ::std::fmt::Display for #name #ty_gen #where_gen {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    f.write_str(#msg)
                }
            }

            impl #impl_gen ::std::error::Error for #name #ty_gen #where_gen {}
        },
        ErrorContents::Struct { fields } => {
            let field_names: Vec<&Ident> = fields.iter().map(|field| &field.name).collect();
            quote! {
                #(#attrs)*
                #[derive(Debug)]
                #[non_exhaustive]
                #vis struct #name {
                    #fields
                }

                impl ::std::fmt::Display for #name {
                    #[allow(unused_variables)]
                    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                        let Self { #(#field_names,)* } = self;
                        f.write_fmt(format_args!(#msg))
                    }
                }

                impl ::std::error::Error for #name {}
            }
        }
        ErrorContents::Enum { sources } => {
            let source_attrs: Vec<&Vec<Attribute>> =
                sources.iter().map(|source| &source.attrs).collect();
            let source_idents: Vec<&Ident> = sources.iter().map(|source| &source.ident).collect();
            quote! {
                #(#attrs)*
                #[derive(Debug)]
                #[non_exhaustive]
                #vis enum #name {
                    #(
                        #(#source_attrs)*
                        #source_idents(#source_idents),
                    )*
                }

                impl ::std::fmt::Display for #name {
                    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                        #write_msg
                    }
                }

                impl ::std::error::Error for #name {
                    fn source(&self) -> ::std::option::Option<&(dyn ::std::error::Error + 'static)> {
                        Some(match self {
                            #(
                                #name::#source_idents(err) => err,
                            )*
                        })
                    }
                }

                #(
                    impl ::std::convert::From<#source_idents> for #name {
                        fn from(source: #source_idents) -> Self {
                            Self::#source_idents(source)
                        }
                    }
                )*
            }
        }
        ErrorContents::Array {
            inner_attrs, inner, ..
        } => quote! {
            #(#attrs)*
            #[derive(Debug)]
            #[non_exhaustive]
            #vis struct #name(#(#inner_attrs)* pub Vec<#inner>);

            impl ::std::fmt::Display for #name {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    f.write_str(#msg)?;
                    f.write_str(":")?;
                    for err in &self.0 {
                        f.write_str("\n")?;
                        f.write_fmt(format_args!("{}", err))?;
                    }
                    Ok(())
                }
            }

            impl ::std::error::Error for #name {}
        },
    })
}

struct Field {
    attrs: Vec<Attribute>,
    vis: Visibility,
    name: Ident,
    colon: Colon,
    ty: Type,
}

impl Parse for Field {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            attrs: input.call(Attribute::parse_outer)?,
            vis: input.parse()?,
            name: input.parse()?,
            colon: input.parse()?,
            ty: input.parse()?,
        })
    }
}

impl ToTokens for Field {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        for attr in &self.attrs {
            attr.to_tokens(tokens);
        }
        self.vis.to_tokens(tokens);
        self.name.to_tokens(tokens);
        self.colon.to_tokens(tokens);
        self.ty.to_tokens(tokens);
    }
}

struct ErrorVariant {
    attrs: Vec<Attribute>,
    ident: Ident,
}

impl Parse for ErrorVariant {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            attrs: input.call(Attribute::parse_outer)?,
            ident: input.parse()?,
        })
    }
}

enum ErrorContents {
    Unit,
    Struct {
        fields: Punctuated<Field, Comma>,
    },
    Enum {
        sources: Punctuated<ErrorVariant, Comma>,
    },
    Array {
        inner_attrs: Vec<Attribute>,
        inner: Type,
    },
}

impl Parse for ErrorContents {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            return Ok(Self::Unit);
        }

        let fork = input.fork();
        if let Ok(fields) = fork.call(Punctuated::parse_terminated) {
            input.advance_to(&fork);
            return Ok(Self::Struct { fields });
        }

        let fork = input.fork();
        if let Ok(sources) = fork.call(Punctuated::parse_terminated) {
            input.advance_to(&fork);
            return Ok(Self::Enum { sources });
        }

        if input.peek(token::Bracket) {
            let content;
            let _ = bracketed!(content in input);
            let attrs = content.call(Attribute::parse_outer)?;
            let inner = content.parse::<Type>()?;
            return Ok(Self::Array {
                inner_attrs: attrs,
                inner,
            });
        }

        Err(input.error("invalid error contents"))
    }
}

struct Error {
    attrs: Vec<Attribute>,
    vis: Visibility,
    name: Ident,
    generics: Generics,
    msg: Option<LitStr>,
    contents: ErrorContents,
}

impl Parse for Error {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse::<Visibility>()?;
        let name = input.parse::<Ident>()?;
        let generics = input.parse::<Generics>()?;
        let msg = input.parse::<LitStr>().ok();
        let contents = input.parse::<ErrorContents>()?;

        if msg.is_none() && !matches!(contents, ErrorContents::Enum { .. }) {
            return Err(input.error("any non-enum error must have a display message"));
        }

        Ok(Self {
            attrs,
            vis,
            name,
            generics,
            msg,
            contents,
        })
    }
}
