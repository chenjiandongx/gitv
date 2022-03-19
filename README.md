# gitv

gitv 是一个由 Rust 编写的 git 仓库分析和可视化的命令行工具。

## 特性

* 支持读取远程仓库和本地仓库
* 支持声明式配置，用户友好，使用成本低；yaml 驱动
* 支持 SQL 查询，并提供额外的自定义函数；依赖 [arrow-datafusion](https://github.com/apache/arrow-datafusion)
* 支持使用 SQL 查询结果可视化数据；依赖 [chartjs](https://www.chartjs.org/)

## 安装

**Cargo 安装**
```shell
$ cargo install gitv
```

**预编译二进制**
```
```

## 使用

命令帮助文档：

```shell
$ gitv -h
gitv 0.1.0

USAGE:
    gitv [OPTIONS] [PATH]

ARGS:
    <PATH>    config file path (default: gitv.yaml)

OPTIONS:
    -c, --create     Retrieve repos and create new databases
    -f, --fetch      Fetch repos metadata from remote source (github)
    -h, --help       Print help information
    -r, --render     Render query result as the given mode (htlm, table)
    -s, --shell      Load data and enter into a new spawn shell
    -V, --version    Print version information
```

gitv 提供 4 种 action（Fetch, Create, Shell, Render）用于同步，拉取，分析和可视化数据。

### 1) Fetch Action

Fetch 负责同步远程数据源的仓库信息并生成一个仓库列表描述文件，用于后续将仓库下载到本地，目前远程数据源只支持 Github。Github 拉取需要 token 验证，所以请在 [settings/token](https://github.com/settings/tokens) 自行申请一个 token（妥善保管好）。

**配置内容**

```yaml
# file: gitv.yaml
# exec: gitv -f ./gitv.yaml
fetch:
  github:
    - cloneDir: "./db"                      # 后续仓库 clone 文件夹，请先提前创建好
      destination: "./db/dongdongx.yaml"    # 生成仓库描述文件路径
      token: "${YOUR_GITHUB_TOKEN}"         # Github Token
      excludeOrgs:                          # 排除记录的 organization 列表
        - "rustlang"
      excludeRepos:                         # 排除记录的仓库列表
        - "chenjiandongx/bar"
      # https://docs.github.com/en/rest/reference/repos#list-repositories-for-the-authenticated-user
      visibility: "owner"                       # Github Api 提供的 visibility 属性
      affiliation: "owner,organization_member"  # Github Api 提供的 affiliation 属性
```

**仓库列表**

```yaml
# file: ./db/dongdongx.yaml
# 字段说明: 
# 1) name: 仓库名称
# 2) branch: 分析的分支（可选，默认为当前分支）
# 3) remote: 仓库远程地址（可选）
# 4) path: 仓库路径
- name: chenjiandongx/magnet-dht
  branch: master
  remote: "https://github.com/chenjiandongx/magnet-dht.git"
  path: "./db/chenjiandongx/magnet-dht"
- name: chenjiandongx/make-it-colorful
  branch: master
  remote: "https://github.com/chenjiandongx/make-it-colorful.git"
  path: "./db/chenjiandongx/make-it-colorful"
- name: chenjiandongx/mandodb
  branch: master
  remote: "https://github.com/chenjiandongx/mandodb.git"
  path: "./db/chenjiandongx/mandodb"
...
```

### 2) Create Action

Create 创建数据库，并提供了 authorMappings 用于映射作者信息，同个项目可能使用不同的账号或者用户名提交。

**配置内容**

```yaml
# file: gitv.yaml
# exec: gitv -c gitv.yaml
create:
  authorMappings:
    - source:
        name: "dongdongx"
        email: "chenjiandongx@qq.com"
      destination:
        name: "chenjiandongx"
        email: "chenjiandongx@qq.com"

  # database 提供两种拉取并分析仓库的方式，分析是最后两者 merge 的结果
  # Note: 注意请不要使用本地已有或者在开发状态下的仓库（有更新操作），建议直接使用一个新文件目录来存储
  # 1) repos: 直接指定 repo 信息，包括 name, branch, remote, path 字段
  # 2) files: fetch action 生成的文件
  databases:
    - path: "./db/dongdongx.csv"        # 数据库文件，实际上为 csv 格式
      repos:
        - name: "gitv"
          remote: "https://github.com/chenjiandongx/gitv"
          path: "./db/chenjiandongx/gitv"
      files:
      - "./db/dongdongx.yaml"
```

**数据文件**

```csv
# file: dongdongx.csv
metric,repo_name,branch,datetime,author_name,author_email,author_domain,tag,ext,insertion,deletion,size,files
COMMIT,chenjiandongx/docs-need-space,master,2018-04-09T23:38:27+08:00,chenjiandongx,chenjiandongx@qq.com,qq.com,,,0,0,0,0
CHANGE,chenjiandongx/docs-need-space,master,2018-04-09T23:38:27+08:00
CHANGE,chenjiandongx/docs-need-space,master,2018-04-09T23:38:27+08:00,chenjiandongx,chenjiandongx@qq.com,qq.com,,cfg,2,0,0,0
COMMIT,chenjiandongx/docs-need-space,master,2018-04-14T13:24:21+08:00,chenjiandongx,chenjiandongx@qq.com,qq.com,,,0,0,0,0
COMMIT,chenjiandongx/docs-need-space,master,2018-04-09T23:44:16+08:00,chenjiandongx,chenjiandongx@qq.com,qq.com,,,0,0,0,0
...
```

**数据采集**

gitv 要求本地环境有 git 工具，通过调用本地 git 命令捕获输出进而得到数据。总共有三类指标：
* COMMIT: 提交数据。
* CHANGE: 提交变动，包括 insertion, deletion 以及 ext。
* TAG: 标签信息（归档），包括 tag, size, files。TAG 不包含提交作者，因为 TAG 是由多个 commit 构成的。

**数据字段**

* metric: 指标类型，包括 COMMIT, CHANGE, TAG
* repo_name: 仓库名称
* branch: 分支名称
* datetime: 提交时间，rfc3339 格式
* author_name: 作者名
* author_email: 作者邮箱
* author_domain: 作者域名，即邮箱后缀，如 @qq.com
* tag: 版本号（TAG）
* ext: 文件扩展名（CHANGE）
* insertion: 变动增加行数（CHANGE）
* deletion: 变动删除函数（CHANGE）
* size: 文件大小，单位字节（TAG）
* files: 文件数量（TAG）

### 3) Shell Action

Shell 创建一个新的 shell 环境并循环读取 SQL 语句进行查询。

**配置文件**

```yaml
# file: gitv.yaml
# exec: gitv -s gitv.yaml
shell:
  executions:
    - tableName: "repo"             # 注册 table 名称
      file: "./db/dongdongx.csv"    # 数据文件路径
```

先敲行 SQL 小试牛刀。
```shell
gitx(sql)> select count(1) as count, author_name from repo where metric='COMMIT' and repo_name='pyecharts/pyecharts' group by author_name order by count desc limit 10;
+-------+---------------+
| count | author_name   |
+-------+---------------+
| 700   | chenjiandongx |
| 286   | chfw          |
| 94    | kinegratii    |
| 47    | jaska         |
| 36    | sunhailinLeo  |
| 25    | LeoSun        |
| 5     | BradyHu       |
| 3     | ayu           |
| 3     | Todd Tao      |
| 3     | Fangyang      |
+-------+---------------+
```

gitv 利用 [arrow-datafusion](https://github.com/apache/arrow-datafusion) 作为查询引擎，datafusion 项目使用了 [Apache Arrow](https://arrow.apache.org/) 作为内存存储格式。Arrow 是 Apache 基金会孵化的一个顶级项目，它主要用来当做跨平台的数据层，为大数据分析加速。

arrow-datafusion 项目目前还在快速发展中，对 SQL 的支持也会越来越完善，除了常用的聚合分析函数 count, min, max, avg 等，gitv 还提供了一些自定义的函数，包括时间函数以及 active 计算函数。

时间函数列表，时间格式为 rfc3339：
| 函数名           | 描述                                           | 输入示例                                        | 输出示例            |
| ---------------- | ---------------------------------------------- | ----------------------------------------------- | ------------------- |
| year             | 计算给定时间的年份                             | 2021-10-12T14:20:50.52+07:00                    | 2021                |
| month            | 计算给定时间的月份                             | 2021-10-12T14:20:50.52+07:00                    | 10                  |
| weekday          | 计算给定时间的星期字符                         | 2021-10-12T14:20:50.52+07:00                    | Mon                 |
| weeknum          | 计算给定时间的星期数字                         | 2021-10-12T14:20:50.52+07:00                    | 0                   |
| hour             | 计算给定时间的小时数                           | 2021-10-12T14:20:50.52+07:00                    | 14                  |
| period           | 计算给定时间的状态（午夜、早上、下午以及晚上） | 2021-10-12T14:20:50.52+07:00                    | Afternoon           |
| timestamp        | 计算给定时间的 Unix 时间戳                     | 2021-10-12T14:20:50.52+07:00                    | 1636960758          |
| timezone         | 计算给定时间的时区                             | 2021-10-12T14:20:50.52+07:00                    | +07:00              |
| duration         | 计算给定时间到现在时间的长度                   | 1647272093                                      | 30hours 2minutes    |
| datetime_format  | 格式化字符串时间                               | 2021-10-12T14:20:50.52+07:00, %Y-%m-%d %H:%M:%S | 2021-10-12 14:20:50 |
| timestamp_format | 格式化时间戳时间                               | 1647272093, %Y-%m-%d %H:%M:%S                   | 2021-10-12 14:20:50 |

SQL 示例：
```shell
gitx(sql)> select timezone(datetime), year(datetime), weekday(datetime), weeknum(datetime), period(datetime) from repo where metric='CHANGE' limit 1;
+-------------------------+---------------------+------------------------+------------------------+-----------------------+
| timezone(repo.datetime) | year(repo.datetime) | weekday(repo.datetime) | weeknum(repo.datetime) | period(repo.datetime) |
+-------------------------+---------------------+------------------------+------------------------+-----------------------+
| +08:00                  | 2018                | Wed                    | 2                      | Afternoon             |
+-------------------------+---------------------+------------------------+------------------------+-----------------------+

gitx(sql)> select repo_name, min(timestamp(datetime)) as ts, timestamp_format(min(timestamp(datetime)), '%Y-%m-%d %H:%M:%S') as created, duration(min(timestamp(datetime))) as duration from repo where metric='COMMIT' and author_name='chenjiandongx' group by repo_name order by ts limit 5;

+------------------------------------+------------+---------------------+------------------------------------+
| repo_name                          | ts         | created             | duration                           |
+------------------------------------+------------+---------------------+------------------------------------+
| chenjiandongx/soksaccounts         | 1491827735 | 2017-04-10 12:35:35 | 4years 11months 8days 4h 52m 33s   |
| chenjiandongx/mmjpg                | 1492145211 | 2017-04-14 04:46:51 | 4years 11months 4days 12h 41m 17s  |
| chenjiandongx/stackoverflow-spider | 1492256146 | 2017-04-15 11:35:46 | 4years 11months 3days 5h 52m 22s   |
| chenjiandongx/mzitu                | 1492433915 | 2017-04-17 12:58:35 | 4years 11months 1day 4h 29m 33s    |
| chenjiandongx/Github-spider        | 1492950023 | 2017-04-23 12:20:23 | 4years 10months 25days 15h 41m 21s |
+------------------------------------+------------+---------------------+------------------------------------+
```

active 计算函数：
| 函数名               | 描述                           | 输入示例                     | 输出示例   |
| -------------------- | ------------------------------ | ---------------------------- | ---------- |
| active_longest_count | 计算最大连续多少天有提交记录   | 2021-10-12T14:20:50.52+07:00 | 2          |
| active_longest_start | 计算最大连续提交天数的起始时间 | 2021-10-12T14:20:50.52+07:00 | 2021-10-12 |
| active_longest_end   | 计算最大连续提交天数的结束时间 | 2021-10-13T14:20:50.52+07:00 | 2021-10-13 |

SQL 示例：
```shell
gitx(sql)> select active_longest_count(datetime), active_longest_start(datetime), active_longest_end(datetime) from repo where metric='COMMIT' and author_name='chenjiandongx';
+-------------------------------------+-------------------------------------+-----------------------------------+
| active_longest_count(repo.datetime) | active_longest_start(repo.datetime) | active_longest_end(repo.datetime) |
+-------------------------------------+-------------------------------------+-----------------------------------+
| 17                                  | 2017-07-17                          | 2017-08-02                        |
+-------------------------------------+-------------------------------------+-----------------------------------+
```

### Render Action

**配置文件**

```yaml
render:
  executions:
    - tableName: "repo"             # 注册 table 名称
      file: "./db/dongdongx.csv"    # 数据文件路径

  display:
    destination: "./"
    renderMode: "html"
    queries:
      - statements:
          - "select count(1) as count, weekday(datetime) as weekday, weeknum(datetime) as weeknum
            from repo
            where metric='COMMIT' and repo_name='pyecharts/pyecharts'
            group by weekday, weeknum order by weeknum;"
        chart:
          name: "commits-on-weekday-pyecharts"
          type: "doughnut"
          width: "720px"
          height: "460px"
          options:
            plugins:
              title:
                display: true
                text: "pyecharts: Commits on weekday"
              datalabels:
                formatter: ${formatter_percent}
                labels:
                  value:
                    color: black
            responsive: false
          data:
            labels:
              - "${weekday}"
            datasets:
              - data:
                  - "${count}"
                label: "pyecharts"
                backgroundColor: "${random}"
```
