error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    links {
    }

    foreign_links {
        Io(::std::io::Error);
        SerdeJson(::serde_json::Error);
        Serenity(::serenity::Error);
        Base64(::base64::DecodeError);
    }

    errors {
    }
}