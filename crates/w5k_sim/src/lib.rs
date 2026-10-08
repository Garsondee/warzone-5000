//! `w5k_sim`: the scheduler and the **integration spine**.
//!
//! The spine is the first-light scenario ([`scenario::first_light`]): one vehicle, one course, one driver, run through the real
//! contract interfaces and recorded as a replay. It runs on stand-ins today (`w5k_contract::testing`); as lanes land real
//! parts, ARCH swaps the stand-ins for them one at a time and the scenario keeps passing. It runs on every merge.
//!
//! * [`scenario`]: the runner, the result, the summary.
//! * [`cli`]: `w5k scenario first-light --out DIR`.

pub mod cli;
pub mod scenario;
