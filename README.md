## Gust

Git is the most popular version control system in the world, and developers usually use GitHub or GitLab to host their codes. Most open source communities base on the services of GitHub to collaborate, track issues and share codes. GitHub's an excellent service to manage all things of open source projects, but it's centralized. GitHub has many ways to influence or limit open source communities. Although the developers have the copyright of the codes, they may not access their repositories. Gust wants to give the communities complete control of their assets.

Gust aims to build self-host service of version control with Git protocol, which can manage and share codes. The Gust is upstream and syncing data to GitHub or other services. Everyone can submit without a signup account and only sign CLA or DCO. There are multiple challenges, particularly syncing the code, PRs and Issues with GitHub, which means it will release when GitHub APIs change and redeploy frequently.

Usually, the open source developers use GitHub flow or Git flow to collaborate on GitHub, and modules divide into multiple repositories. But the monolithic repo and trunk-based development have more benefits[1] for open source communities. All members can focus on the primary functions and release schedule like companies developers on Google, Facebook, and Twitter use monorepo to accelerate the development process. So, Gust provides Git host service of monorepo and trunk-based development.

### 1. Features

#### 1.1 A monolithic repo for all projects

#### 1.2 A decentralized communication for developers

#### 1.3 A synchronized mechanism to connect with GitHub and GitLab

### 2. Architecture

### 3. Developing Status

### 4. Getting Started

### 5. Contributing

### 6. License

[MIT LICENSE](LICENSE) @ Open Rust Initiative

### 7. References

[1] [What is monorepo? (and should you use it?)](https://semaphoreci.com/blog/what-is-monorepo)

[2] [action-rs](https://actions-rs.github.io)
