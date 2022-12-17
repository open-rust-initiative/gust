# Gust 
##  Monorepo Platform for DevOps and Open Source Supply Chain

Git is a content-addressable filesystem and a distributed collaboration system. All files of a single repository persisted on the disk of the machine. It brings a lot of benefits to performance and maintenance. But it also has challenges for monorepo. It is hard to manage a vast code repository like a repo has 20 TB, which is typical in a middle size enterprise.

Google has a monolithic repository platform, Piper, with more than 100 TB of data. It's building on top of Google's infrastructure. Gust's purpose is to imitate Piper's architecture to implement a developing platform which compatible Git and trunk-based development flow for collaboration, open source compliance and supply chain management and DevSecOps.

### 1. Theory of Git

In Git, the content of the file or commit message to store in a file with a specification format, and we call the file an Object. There are four object types: Blob, Tree, Commit and Tag. 

### 2. Gust's features

#### 2.1 Monorepo for Trunk-based Development
 
#### 2.2 Management for Open Source Compliance and Open Source Supply Chain

#### 2.3 Decentralized Communication for Collaboration

#### 2.4 Synchronized Mechanism between Open Source and Inner Source

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

Gust is licensed under this Licensed:

* MIT LICENSE ( [LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)

### 7. References

[1] [What is monorepo? (and should you use it?)](https://semaphoreci.com/blog/what-is-monorepo)
    
[2] [Monorepo: A single repository for all your code](https://medium.com/@mattklein123/monorepo-a-single-repository-for-all-your-code-86a852bff054)

[3] [Why Google Stores Billions of Lines of Code in a Single Repository](https://cacm.acm.org/magazines/2016/7/204032-why-google-stores-billions-of-lines-of-code-in-a-single-repository)

[4] [Trunk Based Development](https://trunkbaseddevelopment.com)