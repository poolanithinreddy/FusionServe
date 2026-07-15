# Tiny image for the stdlib-only mock backends.
FROM python:3.11-slim
WORKDIR /app
COPY tests/mocks/ tests/mocks/
# No dependencies to install — mocks use only the standard library.
ENTRYPOINT ["python3"]
