## Gust - Monorepo Platform for DevOps and Open Source Supply Chain

Git is a distributed system and a content-addressable filesystem. All files of a single repository persisted on the disk of the machine. It brings a lot of benefits of performance and maintain. But also challenges for monorepo. It is hard to manage a vast repository like a repo has 20 TB, which is typical in a middle size enterprise.

Google has a monolithic repository system named Piper which is implemented on top of standard Google infrastructure. The gust purpose implements a monorepo platform on top of Git and Cloud Native architect in Rust.

### 1. Theory of Git

### 2. Gust's features

1. A monorepo for enterprise
2. A monorepo for Open Source Supply Chain
3. A decentralized communication for developers
4. A synchronized mechanism to connect with GitHub and GitLab

### 3. Architecture

### 4. Getting Started

### 5. Contributing

This project enforce the [DCO](https://developercertificate.org).

Contributors sign-off that they adhere to these requirements by adding a Signed-off-by line to commit messages.

```bash
This is my commit message

Signed-off-by: Random J Developer <random@developer.example.org>
```

Git even has a -s command line option to append this automatically to your commit message:

```bash
$ git commit -s -m 'This is my commit message'
```

### 6. License

Freighter is licensed under this Licensed:

* MIT LICENSE ( [LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)

### 7. References

[1] [What is monorepo? (and should you use it?)](https://semaphoreci.com/blog/what-is-monorepo)
    
[2] [Monorepo: A single repository for all your code](https://medium.com/@mattklein123/monorepo-a-single-repository-for-all-your-code-86a852bff054)

[3] [Why Google Stores Billions of Lines of Code in a Single Repository](https://cacm.acm.org/magazines/2016/7/204032-why-google-stores-billions-of-lines-of-code-in-a-single-repository)
