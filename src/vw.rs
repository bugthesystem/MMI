//! `mmi vw` — inspired by [volkswagen](https://github.com/auchenberg/volkswagen).
//!
//! Behaves like `mmi check`, except it quietly passes when it detects a CI
//! environment. Use ironically. Or to make a point. Never to ship.

use std::env;

/// Environment variables set by common CI providers. Presence (with a
/// non-empty value) is treated as "we're being watched".
const CI_ENV_VARS: &[&str] = &[
    "CI",
    "CONTINUOUS_INTEGRATION",
    "GITHUB_ACTIONS",
    "GITLAB_CI",
    "CIRCLECI",
    "TRAVIS",
    "JENKINS_URL",
    "JENKINS_HOME",
    "BUILDKITE",
    "DRONE",
    "TEAMCITY_VERSION",
    "TF_BUILD",
    "BITBUCKET_BUILD_NUMBER",
    "APPVEYOR",
    "SEMAPHORE",
    "CODEBUILD_BUILD_ID",
    "WERCKER",
];

pub fn is_ci() -> bool {
    CI_ENV_VARS
        .iter()
        .any(|k| env::var(k).is_ok_and(|v| !v.is_empty()))
}
