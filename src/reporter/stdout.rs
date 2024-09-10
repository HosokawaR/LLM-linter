use crate::core::{Indications, Reporter};

pub struct StdoutReporter {}

impl Reporter for StdoutReporter {
    async fn report(&self, indications: Indications) {
        for indication in indications.exclude_cancel().exclude_warnings().values {
            println!(
                "{}\nfrom {} to {}\nkind: {:?}\n{}\n",
                indication.location.path,
                indication.location.start_line,
                indication.location.end_line,
                indication.kind,
                indication.message
            );
        }
    }
}

impl StdoutReporter {
    pub fn new() -> StdoutReporter {
        StdoutReporter {}
    }
}
