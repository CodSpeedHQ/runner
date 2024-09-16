use std::{collections::HashMap, env::consts::ARCH};

use lazy_static::lazy_static;

lazy_static! {
    pub static ref BASE_INJECTED_ENV: HashMap<&'static str, String> = {
        HashMap::from([
            ("PYTHONMALLOC", "malloc".into()),
            ("PYTHONHASHSEED", "0".into()),
            ("ARCH", ARCH.into()),
            ("CODSPEED_ENV", "runner".into()),
        ])
    };
}
