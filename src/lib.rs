#![doc = include_str!("../README.md")]

/// This macro allows quickly defining errors in the format that this crate produces.
///
/// It has 4 major forms:
/// - Unit struct:
/// ```ignore
/// quickerr! {
///     ## MyUnitError
///     "it's a unit error"
/// }
/// ```
/// - Record struct:
/// ```ignore
/// quickerr! {
///     ## MyStructError
///     "it's a struct! Field 2 is {field2:?}"
///     - field: Type
///     - field2: Type2
/// }
/// ```
/// - Enum:
/// ```ignore
/// quickerr! {
///     ## MyEnumError
///     "it's a whole enum"
///     - SourceError1
///     - MyUnitError
///     - MyStructError
/// }
/// ```
/// - Transparent enum:
/// ```ignore
/// quickerr! {
///     ## QuietAsAMouse
///     - MyEnumError
///     - REALLY_LOUD_ERROR
/// }
/// ```
///
/// Each form implements `Debug`, `Error`, and `From` as appropriate. The enum forms implement
/// [`std::error::Error::source()`] for each of their variants, and each variant must be the name
/// of an existing error. The struct form exposes the fields for use in the error message.
/// The transparent enum form does not append a message, and simply passes the source along
/// directly. All forms are `#[non_exhaustive]` and all fields are public. They can be made public
/// by adding `pub` to the name like `# pub MyError`.
///
/// Additional attributes can be added before the name to add them to the error type,
/// like so (simply drop the `#[]` part):
/// ```ignore
/// quickerr! {
///     derive(PartialEq, Eq)
///     ## AttrsError
///     "has attributes!"
/// }
/// ```
#[macro_export]
macro_rules! quickerr {
    (
        $($attrs:meta)*
        # $pub:vis $name:ident
        $(
            - $source:ident
        )+
    ) => {
        #[$($attrs)*]
        #[derive(Debug)]
        #[non_exhaustive]
        $pub enum $name {
            $(
                $source ($source),
            )+
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(
                        Self::$source(err) => ::std::fmt::Display::fmt(err, f),
                    )+
                }
            }
        }

        impl ::std::error::Error for $name {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(match self {
                    $(
                        $name::$source(err) => err,
                    )+
                })
            }
        }

        $(
            impl ::std::convert::From<$source> for $name {
                fn from(source: $source) -> Self {
                    Self::$source(source)
                }
            }
        )+
    };

    (
        $($attrs:meta)*
        # $pub:vis $name:ident
        $msg:literal
    ) => {
        #[$($attrs)*]
        #[derive(Debug)]
        #[non_exhaustive]
        $pub struct $name;

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str($msg)
            }
        }

        impl ::std::error::Error for $name {}
    };

    (
        $($attrs:meta)*
        # $pub:vis $name:ident
        $msg:literal
        $(
            - $field:ident : $ty:ty
        )+
    ) => {
        #[$($attrs)*]
        #[derive(Debug)]
        #[non_exhaustive]
        $pub struct $name {
            $(
                pub $field: $ty,
            )+
        }

        impl ::std::fmt::Display for $name {
            #[allow(unused_variables)]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let Self { $($field,)+ } = self;
                f.write_fmt(format_args!($msg))
            }
        }

        impl ::std::error::Error for $name {}
    };

    (
        $($attrs:meta)*
        # $pub:vis $name:ident
        $msg:literal
        $(
            - $source:ident
        )+
    ) => {
        #[$($attrs)*]
        #[derive(Debug)]
        #[non_exhaustive]
        $pub enum $name {
            $(
                $source ($source),
            )+
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str($msg)
            }
        }

        impl ::std::error::Error for $name {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(match self {
                    $(
                        $name::$source(err) => err,
                    )+
                })
            }
        }

        $(
            impl ::std::convert::From<$source> for $name {
                fn from(source: $source) -> Self {
                    Self::$source(source)
                }
            }
        )+
    };
}

#[test]
fn four_forms_compile() {
    quickerr! {
        derive(PartialEq)
        # pub UnitError
        "has no data"
    }

    quickerr! {
        derive(PartialEq)
        # EnumError
        "has error variants"
        - UnitError
    }

    quickerr! {
        derive(PartialEq)
        # pub RecordError
        "has data"
        - field: i32
    }

    quickerr! {
        derive(PartialEq)
        # TransError
        - RecordError
        - EnumError
    }

    let trans = TransError::EnumError(EnumError::UnitError(UnitError));
    let error = format!("{trans}");
    assert_eq!(error, "has error variants")
}
