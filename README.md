# gitv

[![Version info](https://img.shields.io/crates/v/gitv.svg)](https://crates.io/crates/gitv)
[![Version info](https://img.shields.io/badge/License-MIT-brightgreen.svg)](https://opensource.org/licenses/MIT)

gitv 是一个由 Rust 编写的 git 仓库分析和可视化的命令行工具。

![](https://user-images.githubusercontent.com/19553554/162578481-1df8ee5b-42c4-4a11-b0b9-690f702f922d.png)

## 💡 Design

在参与开源的第五个年头，想看看这些年来自己的成长变化，因此需要一个工具来辅助我分析我的代码记录。我希望这个工具拥有以下特性

1. 依赖轻量：gitv 不依赖任何外部组件，仅一个二进制执行文件。
2. 查询灵活：gitv 使用 [arrow-datafusion](https://github.com/apache/arrow-datafusion) 执行引擎进行 SQL 查询，并提供了内置的自定义函数。
3. 用户友好：gitv 使用 yaml 作为其配置格式，并提供了 `-g` flag 快速生成一个配置文件模板。
4. 数据通用：gitv 使用 csv 作为数据文件存储格式，允许用户使用任何其他熟悉的工具来进行数据分析（Pandas, Excel, Tableau...）
5. 集成 Github：gitv 提供了多个 Github Repos 拉取接口，无须手动指定每个仓库信息。
6. 可视化：gitv 使用了 [chartjs](https://www.chartjs.org/) 作为可视化依赖，且支持常用图表的所有配置项。
7. **Rust!**

👉 [《我的开源报告》](https://gitstats.chenjiandongx.me) -- by dongdongx

## 🔰 Installation

**Cargo 安装**

```shell
$ cargo install gitv
```

**预编译二进制**

* [gitv/releases](https://github.com/chenjiandongx/gitv/releases)

## 🔖 Usages

命令帮助文档：

```shell
$ gitv -h
gitv 0.1.0

A git repos analyzing and visualizing tool built in Rust.

USAGE:
    gitv [OPTIONS] [PATH]

ARGS:
    <PATH>    config file path (default: gitv.yaml)

OPTIONS:
    -c, --create       Retrieve repos and create new databases
    -f, --fetch        Fetch repos metadata from remote source (github)
    -g, --gernerate    Generate the example config file (default: gitv.example.yaml)
    -h, --help         Print help information
    -r, --render       Render query result as the given mode (htlm, table)
    -s, --shell        Load data and enter into a new spawn shell
    -V, --version      Print version information
```

gitv 提供多种 action（Fetch, Create, Shell, Render, Generate）用于同步，拉取，分析和可视化数据。

### Fetch Action

Fetch 负责同步远程数据源的仓库信息并生成一个仓库列表文件，用于后续将仓库下载到本地，目前远程数据源只支持 Github。Github 拉取需要 token 验证，所以请在 [settings/token](https://github.com/settings/tokens) 自行申请一个 token（妥善保管好）。

**配置内容：**
```yaml
# 目前支持 githubAuthenticated、githubUser、githubOrg，按需填写
fetch:
  # https://docs.github.com/en/rest/reference/repos#list-repositories-for-the-authenticated-user
  # 拉取 Token 本身账户的仓库列表，可以拉取到 private 仓库
  githubAuthenticated:
    - cloneDir: "./db" # 项目 clone 路径
      destination: "./db/repos.yaml"  # repos 列表文件生成路径
      token: "${YOUR_GITHUB_TOKEN}"   # Github Token
      #
      #（可选项）排除某些 orgs
      # excludeOrgs:
      #   - "some_orgs"
      #
      #（可选项）排除某些项目
      # excludeRepos:
      #   - "some_repos"
      #
      visibility: "owner"
      affiliation: "owner,organization_member"

  # https://docs.github.com/en/rest/reference/repos#list-repositories-for-a-user
  # 拉取某个 Github 用户的仓库列表
  githubUser:
    - cloneDir: "./db"
      destination: "./db/repos-${user}.yaml"
      username: "chenjiandongx" # 拉取的用户名
      token: "${YOUR_GITHUB_TOKEN}"
      #
      #（可选项）排除某些项目
      # excludeRepos:
      #   - "some_repos"
      #
      type: "owner"

  # https://docs.github.com/en/rest/reference/repos#list-organization-repositories
  # 拉取某个 Github Org 的仓库列表
  githubOrg:
    - cloneDir: "./db"
      destination: "./db/repos-${org}.yaml"
      token: "${YOUR_GITHUB_TOKEN}"
      #
      #（可选项）排除某些项目
      # excludeRepos:
      #   - "some_repos"
      #
      org: "pyecharts"  # 拉取的仓库名
      type: ""
```

### Create Action

Create Action 将会在 `databases.dir` 目录下创建 4 个文件，分别为 `active.csv`，`commit.csv`，`change.csv` 以及 `snapshot.csv`。

**active.csv**: 项目活跃指标，目前只记录 Github Stars 和 Github Forks

| 字段      | 描述       | 示例               |
| --------- | ---------- | ------------------ |
| repo_name | 仓库名称   | chenjiandongx/gitv |
| stars     | stars 数量 | 1024               |
| forks     | forks 数量 | 1024               |

```csv
❯ 🐶 cat active.csv | head
repo_name,forks,stars
chenjiandongx/ginprom,52,107
chenjiandongx/kubectl-images,16,154
...
```

**commit.csv**: 项目提交信息

| 字段          | 描述                | 示例                                     |
| ------------- | ------------------- | ---------------------------------------- |
| repo_name     | 仓库名称            | chenjiandongx/gitv                       |
| hash          | 提交 hash           | 5c1e21ff11b0b0d819de09f689f077be1cdd6416 |
| branch        | 扫描分支            | master                                   |
| datetime      | 提交时间（rfc3339） | 2017-05-07T21:23:26+08:00                |
| authore_name  | 作者名称            | chenjiandongx                            |
| author_email  | 作者邮箱            | chenjiandongx@qq.com                     |
| author_domain | 邮箱域名            | qq.com                                   |

```csv
❯ 🐶 cat commit.csv | head
repo_name,hash,branch,datetime,author_name,author_email,author_domain
chenjiandongx/Github-spider,5c1e21ff11b0b0d819de09f689f077be1cdd6416,master,2017-05-07T21:23:26+08:00,chenjiandongx,chenjiandongx@qq.com,qq.com
chenjiandongx/Github-spider,309121d6f41c8817cdd8189834834009af452f09,master,2017-05-04T00:25:38+08:00,chenjiandongx,chenjiandongx@qq.com,qq.com
...
```

**change.csv**: 项目代码变更信息

| 字段          | 描述                | 示例                                     |
| ------------- | ------------------- | ---------------------------------------- |
| repo_name     | 仓库名称            | chenjiandongx/gitv                       |
| hash          | 提交 hash           | 5c1e21ff11b0b0d819de09f689f077be1cdd6416 |
| branch        | 扫描分支            | master                                   |
| datetime      | 提交时间（rfc3339） | 2017-05-07T21:23:26+08:00                |
| authore_name  | 作者名称            | chenjiandongx                            |
| author_email  | 作者邮箱            | chenjiandongx@qq.com                     |
| author_domain | 邮箱域名            | qq.com                                   |
| ext           | 文件后缀            | rs                                       |
| insertion     | 代码增加行数        | 1024                                     |
| deletetion    | 代码删除函数        | 1024                                     |

```csv
❯ 🐶 cat change.csv | head
repo_name,hash,branch,datetime,author_name,author_email,author_domain,ext,insertion,deletion
chenjiandongx/Github-spider,5c1e21ff11b0b0d819de09f689f077be1cdd6416,master,2017-05-07T21:23:26+08:00,chenjiandongx,chenjiandongx@qq.com,qq.com,py,0,15
chenjiandongx/Github-spider,309121d6f41c8817cdd8189834834009af452f09,master,2017-05-04T00:25:38+08:00,chenjiandongx,chenjiandongx@qq.com,qq.com,md,24,24
...
```

**snaphost.csv**: 项目文件快照信息

| 字段      | 描述                | 示例                      |
| --------- | ------------------- | ------------------------- |
| repo_name | 仓库名称            | chenjiandongx/gitv        |
| branch    | 扫描分支            | master                    |
| datetime  | 提交时间（rfc3339） | 2017-05-07T21:23:26+08:00 |
| ext       | 文件后缀            | rs                        |
| code      | 代码行数            | 1024                      |
| comments  | 注释行数            | 1024                      |
| blanks    | 空格行数            | 1024                      |

```csv
❯ 🐶 cat snapshot.csv | head
repo_name,branch,datetime,ext,code,comments,blanks
chenjiandongx/Github-spider,master,2017-05-07T21:23:26+08:00,markdown,0,141,47
chenjiandongx/Github-spider,master,2017-05-07T21:23:26+08:00,python,338,97,107
```

**配置内容：**
```yaml
create:
  # 不执行 git pull 命令，只执行 git clone，如果项目不存在的话
  disablePull: false
  #
  # （可选项）作者映射关系，因为可能出现同个作者使用了不同的名称或者账号
  # authorMappings:
  #   - source:
  #       name: "dingdongx"
  #       email: "chenjiandongx@qq.com"
  #     destination:
  #       name: "chenjiandongx"
  #       email: "chenjiandongx@qq.com"
  #
  # 数据库信息
  databases:
    - dir: "./db" # 数据将存放到路径，需自己提前创建好
      # 最终扫描的仓库是 files + repos 的 merge 结果
      # 如若只想扫描本地的某几个仓库，可以使用直接指定 repos 的方式
      # 如若想扫描 Github 账号下的仓库，则推荐使用 `fetch` 命令生成的仓库文件
      #
      #（可选项）仓库列表文件，由 `fetch` 命令创建，文件内容同 `repos` 属性
      # files:
      #   - "./db/repos.yaml"
      #
      #（可选项）仓库列表
      # repos:
      #   - name: "chenjiandongx/gitv"
      #     branch: "master"  # 扫描的分支
      #     path: "~/src/github.com/chenjiandongx/gitv"
      #     remote: "https://github.com/chenjiandongx/gitv"
```

### Shell Action

Shell 读取数据并创建一个新的 shell 环境并循环读取 SQL 语句进行查询。读取的数据为 `Create Action` 创建的多个文件，并一一映射为数据库 table。

arrow-datafusion 项目目前还在快速发展中，对 SQL 的支持也会越来越完善，除了常用的聚合分析函数 count, min, max, avg 等，gitv 还提供了一些自定义的函数，包括时间函数以及 active 计算函数。

**时间函数列表：**

| 函数名            | 描述                                           | 输入示例                     | 输出示例                     |
| ----------------- | ---------------------------------------------- | ---------------------------- | ---------------------------- |
| year              | 计算给定时间的年份                             | 2021-10-12T14:20:50.52+07:00 | 2021                         |
| month             | 计算给定时间的月份                             | 2021-10-12T14:20:50.52+07:00 | 10                           |
| weekday           | 计算给定时间的星期字符                         | 2021-10-12T14:20:50.52+07:00 | Mon                          |
| weeknum           | 计算给定时间的星期数字                         | 2021-10-12T14:20:50.52+07:00 | 0                            |
| hour              | 计算给定时间的小时数                           | 2021-10-12T14:20:50.52+07:00 | 14                           |
| period            | 计算给定时间的状态（午夜、早上、下午以及晚上） | 2021-10-12T14:20:50.52+07:00 | Afternoon                    |
| timestamp         | 计算给定时间的 Unix 时间戳                     | 2021-10-12T14:20:50.52+07:00 | 1636960758                   |
| timezone          | 计算给定时间的时区                             | 2021-10-12T14:20:50.52+07:00 | +07:00                       |
| duration          | 计算给定时间到现在时间的长度                   | 1647272093                   | 30hours 2minutes             |
| timestamp_rfc3339 | 格式化时间戳时间                               | 1647272093                   | 2021-10-12T14:20:50.52+07:00 |

**active 计算函数：**

| 函数名               | 描述                           | 输入示例                     | 输出示例   |
| -------------------- | ------------------------------ | ---------------------------- | ---------- |
| active_longest_count | 计算最大连续多少天有提交记录   | 2021-10-12T14:20:50.52+07:00 | 2          |
| active_longest_start | 计算最大连续提交天数的起始时间 | 2021-10-12T14:20:50.52+07:00 | 2021-10-12 |
| active_longest_end   | 计算最大连续提交天数的结束时间 | 2021-10-13T14:20:50.52+07:00 | 2021-10-13 |

**配置内容：**
```yaml
shell:
  executions:
    - dbName: "db"  # 数据库名称
      dir: "./db"   # 数据文件所在目录
```

SQL 示例：
```shell
# 使用 commit.csv 的数据，被注册为 `commit` table，数据库名称在 executions 中指定
gitx(sql)> select repo_name, year(datetime) as year, timezone(datetime) as tz from 'db.commit' limit 1;
+-----------------------------+------+--------+
| repo_name                   | year | tz     |
+-----------------------------+------+--------+
| chenjiandongx/Github-spider | 2017 | +08:00 |
+-----------------------------+------+--------+
Query OK, elapsed: 1.77555ms

# 使用 change.csv 的数据，被注册为 `change` table
gitx(sql)> select ext, max(insertion) as insertion from 'db.change' group by ext order by insertion desc limit 1;
+------+-----------+
| ext  | insertion |
+------+-----------+
| json | 742057    |
+------+-----------+
Query OK, elapsed: 16.361255ms

# 使用 tag.csv 的数据，被注册为 `tag` table
gitx(sql)> select * from 'db.tag' where year(datetime) <= 2017 limit 1;
+-------------------------+--------+---------------------------+--------+
| repo_name               | branch | datetime                  | tag    |
+-------------------------+--------+---------------------------+--------+
| chenjiandongx/pytreemap | master | 2017-11-28T23:20:16+08:00 | v0.0.1 |
+-------------------------+--------+---------------------------+--------+
Query OK, elapsed: 2.56332ms

# 使用 active.csv 的数据，被注册为 `active` table
gitx(sql)> select * from 'db.active' where repo_name='chenjiandongx/sniffer';
+-----------------------+-------+-------+
| repo_name             | forks | stars |
+-----------------------+-------+-------+
| chenjiandongx/sniffer | 33    | 430   |
+-----------------------+-------+-------+
Query OK, elapsed: 2.156542ms
```

### Render Action

Render 负责根据配置执行 SQL 语句并渲染 chartjs 图表。
 
**配置内容：**
```yaml
render:
  executions:
    - dbName: "db"
      dir: "./db"

  #（可选项）自定义颜色列表
  # colors: 
  #   Blues: ["#deebf7", "#c6dbef", "#9ecae1", "#6baed6", "#4292c6", "#2171b5", "#08519c", "#08306b"]
  #
  #（可选项）自定义 js 函数，使用 `{{% %}}` 包裹起来
  # functions:
  #   my_function_name: "{{% function (value,context){return Math.round(value*100)/100} %}}"

  display:
    destination: "./gitstats/static"  # 图表生成路径
    renderMode: "html"  # 渲染格式，有 html/table 可选
    #
    # （可选项）依赖资源文件，也可以指定为本地依赖
    # dependency:
    #   chartjs: "https://cdn.bootcdn.net/ajax/libs/Chart.js/3.7.1/chart.min.js"
    #   datalabels: "https://cdn.jsdelivr.net/npm/chartjs-plugin-datalabels@2.0.0"
    #
    # 执行查询语句和图表生成样式
    queries:
      - statements: # sql 语句
          - "SELECT repo_name, stars from 'db.active' limit 5;"
        # chart 为 chartjs 的配置字段，完全遵照 chartjs 的配置格式
        # https://www.chartjs.org/docs/latest/
        chart:
          name: "project-active" # html 文件名称（请使用英文命名）
          type: "bar"
          width: "680px"
          height: "460px"
          options:
            animation:
              duration: 0
            plugins:
              title:
                display: true
                text: "title here"
              datalabels:
                formatter: ${my_function_name}
            responsive: false
          data:
            labels:
              - "${repo_name}" # ${field} -> field 会被替换成 sql 中的同名字段数据
            datasets:
              - data:
                  - "${stars}" # ${field} -> field 会被替换成 sql 中的同名字段数据
                label: "project count"
                backgroundColor: "${Blues}" # 替换 colors 中定义的颜色列表
```

除了可使用自己定义的颜色列表和函数列表，也可以使用 gitv 提供的内置颜色和函数。

* 颜色列表：[colors.yaml](./static/colors.yaml)
* 函数列表：[functions.yaml](./static/functions.yaml)

更多使用示例请参考 [./gitstats](./gitstats/) 目录或访问 [gitstats.chenjandongx.me](https://gitstats.chenjiandongx.me)

## 📋 License

MIT [©chenjiandongx](https://github.com/chenjiandongx)
