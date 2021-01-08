use std::io::Write;

use env_logger;

pub fn init_testing() {
    env_logger::builder()
        .format(|buf, record| writeln!(buf,
                                       "[{} {} {}:{}] {}",
                                       buf.timestamp(),
                                       record.level(),
                                       record.file().unwrap_or(record.target()),
                                       record.line().unwrap_or(0),
                                       record.args()))
        .is_test(true)
        .try_init();
}
