# Security policy

## Supported versions

Security fixes are made for the latest release only. Please update before reporting.

| Version | Supported |
|---|---|
| Latest release | Yes |
| Older releases | No |

## Reporting a vulnerability

Please **do not** open a public issue for security problems.

1. Go to the repository's **Security** tab and choose **Report a vulnerability** to send a private report.
2. If that option is not available, open an issue titled "Security contact request" **without any details**, and the maintainer will reply with a private way to talk.

Include what you can: the affected component (browser extension or Windows app), the version, steps to reproduce, and the impact you expect. A proof of concept helps, but never share private documents.

You can expect an acknowledgement within 7 days. Once a fix is released, you will be credited in the release notes unless you prefer otherwise.

## Scope

Areas of particular interest:

- Handling of `praydf:` links (for example network paths, path tricks or other schemes that get past validation).
- Documents that crash or hang the Windows app or the extension.
- Anything that sends document content off the device.
