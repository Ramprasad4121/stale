from setuptools import setup, find_packages

setup(
    name="stale-enterprise",
    version="1.0.0",
    description="Enterprise SDK for Stale - fail-closed guardrails for agents that touch money",
    long_description=open("README.md").read() if __import__("os").path.exists("README.md") else "",
    long_description_content_type="text/markdown",
    author="Stale Enterprise",
    author_email="ramprasad@stale.sh",
    url="https://github.com/Ramprasad4121/stale",
    packages=find_packages(),
    install_requires=[
        "requests>=2.28.0",
    ],
    python_requires=">=3.8",
    keywords=["defi", "guardrails", "security", "chainlink", "enterprise", "ai-agents"],
    classifiers=[
        "Development Status :: 5 - Production/Stable",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: MIT License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
    ],
)
