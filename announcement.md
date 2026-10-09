The text in this RFC has been replaced with a new version written by @Bryanskiy, me and @aerooneqq based on the [implementation](https://github.com/rust-lang/rust/pulls?q=is%3Apr+state%3Amerged+label%3AF-fn_delegation) of [this feature](https://github.com/rust-lang/rust/issues/118212) in nightly rustc.

The old version of the RFC is still available [here](https://github.com/Bryanskiy/rust-delegation-proposal/blob/main/0000-fn-delegation-old.md).
Most of the high-level design of the old RFC has been preserved both in the new RFC and in the implementation.

An accompanying implementation experience report covering more technical details can also be found [here](https://github.com/Bryanskiy/rust-delegation-proposal/blob/main/exp-report.md), but it is in a draft state, covers only some aspects of the implementation, and has been partially merged into the RFC. We don't plan to extend it further.

The text still contains a number of TODOs in the rationale appendices. I'll gradually fill them in the next few weeks together with addressing incoming feedback.

LLM disclosure - an LLM was used for proofreading; the changes were reviewed by me:
- Fixing English language issues (spelling, punctuation, grammar, normalizing to American English)
- Checking that sections are ordered consistently, links are alive and styled consistently, code examples compile, and defined terminology is applied consistently
