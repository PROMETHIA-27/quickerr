# quickerr

A macro to define errors quickly, like `thiserror` but terser and more opinionated. Exclusively uses a decl macro, so compile times should not be greatly impacted. It uses markdown-like syntax. Primarily for my own personal use as I find it extremely helpful for quickly defining high quality errors.

## Example:
```rust
# use quickerr::quickerr;
# quickerr! { MyOtherError "" }
# quickerr! { MySecondOtherError "" }
quickerr! {
  pub EnumError
  "a problem happened!"
  - MyOtherError
  - MySecondOtherError
}
```
this expands to:
```rust,ignore
#[derive(Debug)]
#[non_exhaustive]
pub enum EnumError {
    MyOtherError(MyOtherError),
    MySecondOtherError(MySecondOtherError),
}

impl ::std::fmt::Display for EnumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a problem happened!")
    }
}

impl ::std::error::Error for EnumError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            MyOtherError(err) => err,
            MySecondOtherError(err) => err,
        })
    }
}

impl ::std::convert::From<MyOtherError> for EnumError {
    fn from(source: MyOtherError) -> Self {
        Self::MyOtherError(source)
    }
}

impl ::std::convert::From<MySecondOtherError> for EnumError {
    fn from(source: MySecondOtherError) -> Self {
        Self::MySecondOtherError(source)
    }
}
```
