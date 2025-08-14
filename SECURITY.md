# Security Policy

This project uses credentials for integration tests and CI runs.

- **Least privilege**: CI service accounts should only have access to test resources.
- **Rotation**: Rotate secrets periodically and update the GitHub Actions secrets.
- **No logging of secrets**: Avoid printing connection strings or API keys in tests or code.

To rotate credentials, update the secrets in GitHub and refresh your local `.env` files.
