# 停車パターン差分検知システム セットアップガイド

このドキュメントでは、停車パターン差分検知システムをGitHub Actions + ECSで定期実行するための設定手順を説明します。

## 概要

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  GitHub Actions │────▶│   ECS Task      │────▶│  PostgreSQL     │
│  (cron 09:00)   │     │  (detect_stop_  │     │  (VPC内)        │
│                 │     │   patterns)     │     │                 │
└────────┬────────┘     └────────┬────────┘     └─────────────────┘
         │                       │
         │◀──────────────────────┘
         │    CloudWatch Logs経由で結果取得
         ▼
┌─────────────────┐
│  GitHub Issue   │
│  自動作成       │
└─────────────────┘
```

- **GitHub Actions**: 毎日定時にECSタスクを起動し、結果に応じてIssueを作成
- **ECS Task**: VPC内のDBにアクセスして差分を検出
- **GitHub Token不要**: ECS側にGitHub Tokenを配置する必要なし

## 1. AWS側の設定

### 1.1 CloudWatch Logsロググループ作成

```bash
aws logs create-log-group --log-group-name /ecs/detect-stop-patterns
```

### 1.2 ECSタスク定義

以下の内容でタスク定義を作成します。

```json
{
  "family": "detect-stop-patterns",
  "containerDefinitions": [
    {
      "name": "detect-stop-patterns",
      "image": "<AWS_ACCOUNT_ID>.dkr.ecr.ap-northeast-1.amazonaws.com/stationapi:latest",
      "command": ["./detect_stop_patterns", "-o", "all"],
      "environment": [
        {
          "name": "ODPT_API_KEY",
          "value": "<YOUR_ODPT_API_KEY>"
        },
        {
          "name": "DATABASE_URL",
          "value": "postgres://user:pass@host:5432/dbname"
        }
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/detect-stop-patterns",
          "awslogs-region": "ap-northeast-1",
          "awslogs-stream-prefix": "detect-stop-patterns"
        }
      },
      "essential": true
    }
  ],
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "256",
  "memory": "512",
  "executionRoleArn": "<ECS_EXECUTION_ROLE_ARN>",
  "taskRoleArn": "<ECS_TASK_ROLE_ARN>"
}
```

> **Note**: 機密情報（ODPT_API_KEY, DATABASE_URL）はAWS Secrets Managerを使用することを推奨します。

### 1.3 IAMポリシー（GitHub Actions用）

GitHub ActionsからECSタスクを実行するためのIAMポリシーを作成します。

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "ECSRunTask",
      "Effect": "Allow",
      "Action": [
        "ecs:RunTask",
        "ecs:DescribeTasks"
      ],
      "Resource": "*"
    },
    {
      "Sid": "PassRole",
      "Effect": "Allow",
      "Action": "iam:PassRole",
      "Resource": [
        "<ECS_EXECUTION_ROLE_ARN>",
        "<ECS_TASK_ROLE_ARN>"
      ]
    },
    {
      "Sid": "CloudWatchLogs",
      "Effect": "Allow",
      "Action": [
        "logs:GetLogEvents",
        "logs:DescribeLogStreams"
      ],
      "Resource": "arn:aws:logs:ap-northeast-1:*:log-group:/ecs/detect-stop-patterns:*"
    }
  ]
}
```

このポリシーをアタッチしたIAMユーザーを作成し、アクセスキーを発行します。

## 2. GitHub側の設定

### 2.1 Secrets

リポジトリの **Settings → Secrets and variables → Actions → Secrets** で以下を設定:

| 名前 | 説明 |
|------|------|
| `AWS_ACCESS_KEY_ID` | AWSアクセスキーID |
| `AWS_SECRET_ACCESS_KEY` | AWSシークレットアクセスキー |

### 2.2 Variables

同じ画面の **Variables** タブで以下を設定:

| 名前 | 例 | 説明 |
|------|-----|------|
| `AWS_REGION` | `ap-northeast-1` | AWSリージョン |
| `ECS_CLUSTER` | `stationapi-cluster` | ECSクラスター名 |
| `ECS_TASK_DEFINITION` | `detect-stop-patterns` | タスク定義名 |
| `ECS_SUBNETS` | `subnet-xxx,subnet-yyy` | サブネットID（カンマ区切り） |
| `ECS_SECURITY_GROUPS` | `sg-xxx` | セキュリティグループID |
| `ECS_LOG_GROUP` | `/ecs/detect-stop-patterns` | CloudWatch Logsロググループ名 |

### 2.3 Issueラベル作成

**Issues → Labels** で以下のラベルを作成:

- `stop-pattern-change` - 停車パターン変更用
- `automated` - 自動作成されたIssue用

## 3. データベース設定

以下のテーブルが必要です（`create_table.sql` に含まれています）:

```sql
-- スナップショット保存用
CREATE TABLE stop_pattern_snapshots (
    id SERIAL PRIMARY KEY,
    operator_id VARCHAR(100) NOT NULL,
    railway_id VARCHAR(100) NOT NULL,
    train_type_id VARCHAR(100) NOT NULL,
    train_type_name VARCHAR(100),
    station_ids TEXT[] NOT NULL,
    station_names TEXT[],
    captured_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    captured_date DATE NOT NULL,
    UNIQUE(railway_id, train_type_id, captured_date)
);

-- 変更ログ用
CREATE TABLE stop_pattern_changes (
    id SERIAL PRIMARY KEY,
    operator_id VARCHAR(100) NOT NULL,
    railway_id VARCHAR(100) NOT NULL,
    railway_name VARCHAR(100),
    train_type_id VARCHAR(100) NOT NULL,
    train_type_name VARCHAR(100),
    change_type VARCHAR(20) NOT NULL,
    station_id VARCHAR(100) NOT NULL,
    station_name VARCHAR(100),
    detected_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    acknowledged BOOLEAN DEFAULT FALSE,
    acknowledged_at TIMESTAMP
);
```

## 4. 動作確認

### 手動実行

```bash
# GitHub CLIを使用
gh workflow run detect-stop-patterns.yml

# または特定のオペレーターを指定
gh workflow run detect-stop-patterns.yml -f operators=TokyoMetro,JR-East
```

GitHub Actionsタブから「Run workflow」ボタンでも実行可能です。

### ログ確認

```bash
# 最新のワークフロー実行を確認
gh run list --workflow=detect-stop-patterns.yml

# 特定の実行のログを表示
gh run view <RUN_ID> --log
```

## 5. 運用

### 定期実行スケジュール

デフォルトでは毎日 09:00 JST（00:00 UTC）に実行されます。

変更する場合は `.github/workflows/detect-stop-patterns.yml` の cron 設定を編集:

```yaml
on:
  schedule:
    - cron: '0 0 * * *'  # UTC時間で指定
```

### 変更の確認

Issueが作成されたら内容を確認し、対応後にDBを更新:

```sql
-- 確認済みにする
UPDATE stop_pattern_changes
SET acknowledged = TRUE, acknowledged_at = NOW()
WHERE acknowledged = FALSE;
```

### ログローテーション

自動的に以下のローテーションが実行されます:

- **変更ログ**: acknowledged=TRUE かつ 90日経過したものを削除
- **スナップショット**: 30日経過したものを削除

設定変更はCLIオプションで可能:

```bash
./detect_stop_patterns --changes-retention 180 --snapshots-retention 60
```

## トラブルシューティング

### ECSタスクが起動しない

- サブネットがインターネットアクセス可能か確認（NAT Gateway or Public IP）
- セキュリティグループでアウトバウンド443が許可されているか確認
- タスク定義のCPU/メモリ設定を確認

### CloudWatch Logsにログが出ない

- ログストリーム名のプレフィックス設定を確認
- ECS実行ロールにCloudWatch Logs書き込み権限があるか確認

### Issueが作成されない

- ワークフローの実行ログを確認
- リポジトリにIssue作成権限があるか確認（GitHub Actionsのデフォルトトークンで可能）
