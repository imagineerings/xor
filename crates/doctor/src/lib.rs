pub mod checks;

pub use checks::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheckReport>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct DoctorCheckReport {
    pub name: String,
    pub status: DoctorStatus,
    pub message: String,
    pub remediation: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorStatus {
    Pass,
    Warning,
    Fail,
}

pub trait DoctorCheck {
    fn name(&self) -> &str;
    fn run(&self) -> DoctorCheckReport;
}

#[derive(Default)]
pub struct Doctor {
    checks: Vec<Box<dyn DoctorCheck>>,
}

impl Doctor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_check(mut self, check: impl DoctorCheck + 'static) -> Self {
        self.checks.push(Box::new(check));
        self
    }

    pub fn run(&self) -> DoctorReport {
        DoctorReport {
            checks: self.checks.iter().map(|check| check.run()).collect(),
        }
    }
}

impl DoctorReport {
    pub fn has_failures(&self) -> bool {
        self.checks
            .iter()
            .any(|check| check.status == DoctorStatus::Fail)
    }
}

impl DoctorCheckReport {
    pub fn pass(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: DoctorStatus::Pass,
            message: message.into(),
            remediation: None,
        }
    }

    pub fn warning(
        name: impl Into<String>,
        message: impl Into<String>,
        remediation: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            status: DoctorStatus::Warning,
            message: message.into(),
            remediation: Some(remediation.into()),
        }
    }

    pub fn fail(
        name: impl Into<String>,
        message: impl Into<String>,
        remediation: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            status: DoctorStatus::Fail,
            message: message.into(),
            remediation: Some(remediation.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct PassingCheck;

    impl DoctorCheck for PassingCheck {
        fn name(&self) -> &str {
            "passing"
        }

        fn run(&self) -> DoctorCheckReport {
            DoctorCheckReport::pass(self.name(), "ok")
        }
    }

    #[test]
    fn aggregates_check_reports() {
        let report = Doctor::new().with_check(PassingCheck).run();

        assert_eq!(report.checks.len(), 1);
        assert!(!report.has_failures());
    }
}
