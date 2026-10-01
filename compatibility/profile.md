# Rarog compatibility profile

- Schema version: 1
- Profile revision: 1
- Evidence sets: 2

> Evidence entries are historical measurements bound to their own exact Rarog commits.
> The profile revision does not imply that all evidence was measured on one current engine commit.

No aggregate compatibility score or cross-kind status is computed.

## Evidence index

| ID | Kind | Measured Rarog commit | Platform | Raw digest | Normalized digest |
| --- | --- | --- | --- | --- | --- |
| real-web-first-baseline | real-web | `2301370b6ccce06a8839a94a75dfeee12e3a3499` | github-actions-ubuntu-24.04-live | `sha256:daa3bf52900393e3755b903f0ba2fef9a310b3c3294c84869908ee27a1acf2b8` | `sha256:821907e3e5903f6184abd34001559e141a8e0f17f2dd04ac9bcc94245f585053` |
| wpt-first-selected-baseline | wpt | `dcd349dd37b0340ec67a2fb8d36b13980e2fd918` | github-actions-ubuntu-24.04 | `sha256:087d34554cefb38d54d82323a5a8d3419e3a97936fe9b4a2e539e2a2cd6f347f` | `sha256:f02f45dd2b0e0affa7c0bf0fab97f0b1f0dc52c769201af71f4f300a66df3ae0` |

## real-web-first-baseline

- Corpus revision: 1
- Corpus digest: `sha256:835751b5e67c10ac4f77c838d4e4496724597814c9917f58b1358a7d076df203`
- Scenarios: 1

### Real-Web scenario outcomes

| Scenario | Category | Detail |
| --- | --- | --- |
| rfc9110-html | completed-observation | completed |

### Real-Web dependency states

| Scenario | Origin | Role | State |
| --- | --- | --- | --- |
| rfc9110-html | https://www.rfc-editor.org | primary-document | available |

## wpt-first-selected-baseline

- Upstream WPT: `a83afd4402cffdc876508fe9a47f916d4136099f`
- Selection: `sha256:3d0951f045626f699ce123a1cc52e06b989ad1d01305e43f1471f6719163b075`
- Measured tests: 5
- Unexpected tests: 5

### WPT observed statuses

| Status | Count |
| --- | ---: |
| ERROR | 3 |
| FAIL | 2 |
