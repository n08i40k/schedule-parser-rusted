#[macro_export]
macro_rules! or_continue {
    ( $e:expr ) => {
        {
            if let Some(x) = $e {
                x
            } else {
                continue;
            }
        }
    }
}

#[macro_export]
macro_rules! or_break {
    ( $e:expr ) => {
        {
            if let Some(x) = $e {
                x
            } else {
                break;
            }
        }
    }
}