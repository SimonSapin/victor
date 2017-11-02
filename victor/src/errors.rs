use std::io;

macro_rules! error_enum {
    ($( $Variant: ident ($Type: ty), )+) => {
        /// An error returned by Victor.
        #[derive(Debug)]
        pub enum VictorError {
            $(
                $Variant($Type),
            )+
        }

        $(
            impl From<$Type> for VictorError {
                fn from(e: $Type) -> Self {
                    VictorError::$Variant(e)
                }
            }
        )+
    }
}

error_enum! {
    Io(io::Error),
}
