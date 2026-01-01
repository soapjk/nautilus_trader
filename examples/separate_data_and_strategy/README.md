# 数据采集与策略分离架构示例

## 架构说明

这个示例展示了如何将数据采集和策略交易分离到两个独立的进程中：

- **数据采集进程 (`data_producer.py`)**: 连接交易所获取实时数据，通过 Redis Stream 发布
- **策略交易进程 (`strategy_consumer.py`)**: 从 Redis 订阅数据，运行策略，执行交易

**优势**: 策略进程可以随时重启，数据采集进程持续运行不中断。

## 架构图

```
┌─────────────────────────────────────────────────────────────────────┐
│                    数据采集进程 (data_producer.py)                   │
│                                                                       │
│  - 连接交易所获取实时数据                                              │
│  - 通过 Redis Stream 发布数据                                          │
│  - 可选：同时持久化到本地 Parquet                                      │
│                                                                       │
│  交易所WebSocket → DataClient → Redis Stream "market_data"           │
└─────────────────────────────────────────────────────────────────────┘
                                   │
                                   │ Redis Pub/Sub
                                   ↓
┌─────────────────────────────────────────────────────────────────────┐
│                    策略交易进程 (strategy_consumer.py)                │
│                                                                       │
│  - 从 Redis 订阅数据                                                   │
│  - 运行交易策略                                                        │
│  - 发送订单到交易所                                                     │
│                                                                       │
│  Redis Stream → Strategy → ExecutionClient → 交易所订单               │
└─────────────────────────────────────────────────────────────────────┘
```

## 使用说明

### 1. 启动 Redis

```bash
# 使用 Docker Compose
docker-compose up -d

# 或者直接启动 Redis
redis-server
```

### 2. 配置 API 密钥

编辑 `data_producer.py` 和 `strategy_consumer.py`，填入你的 API 密钥：

```python
api_key="your_api_key",
api_secret="your_api_secret",
```

### 3. 启动数据采集进程

```bash
python data_producer.py
```

### 4. 启动策略交易进程

```bash
python strategy_consumer.py
```

### 5. 测试策略热更新

```bash
# 在策略进程终端按 Ctrl+C 停止
# 数据采集进程继续运行

# 修改策略代码后重新启动
python strategy_consumer.py

# 策略立即继续接收实时数据，无需等待
```

## 配置参数详解

### MessageBusConfig 关键参数

| 参数 | 生产者 | 消费者 | 说明 |
|------|--------|--------|------|
| `streams_prefix` | `"market_data"` | - | 设置 Redis stream 名称 |
| `external_streams` | - | `["market_data"]` | 订阅的 stream 列表 |
| `use_trader_id` | `False` | - | 不在 stream 名中添加 trader ID |
| `autotrim_mins` | `60` | - | Redis stream 保留时长（分钟） |
| `encoding` | `"msgpack"` | `"msgpack"` | 序列化格式（必须一致） |
| `buffer_interval_ms` | `100` | - | 批量发送间隔（毫秒） |

### LiveDataEngineConfig 关键参数

| 参数 | 值 | 说明 |
|------|------|------|
| `external_clients` | `[ClientId("BINANCE")]` | 声明数据来自 Redis，不是本地客户端 |

## 监控 Redis 状态

```bash
# 查看 Redis 中的 stream
redis-cli

# 在 redis-cli 中：
> KEYS *           # 查看所有键
> XLEN market_data # 查看 stream 长度
> XRANGE market_data - + COUNT 10  # 查看最近 10 条消息
> XINFO STREAM market_data  # 查看 stream 详细信息
```

## 架构优势

| 特性 | 单进程方案 | 分离方案 |
|------|-----------|---------|
| 策略重启时数据丢失 | ❌ 会丢失 | ✅ 不会丢失 |
| 策略热更新 | ❌ 需要停止节点 | ✅ 独立重启策略进程 |
| 多个策略共享数据 | 需要重复连接 | ✅ 一个数据源服务多个策略 |
| 资源隔离 | 无隔离 | ✅ 完全隔离 |
| 故障影响范围 | 全局 | 局部 |
| 数据持久化 | 可选 | ✅ 独立配置 |

## 高级配置示例

### 多数据源架构

```python
# 数据采集进程可以连接多个交易所
data_clients={
    BINANCE: BinanceDataClientConfig(...),
    BYBIT: BybitDataClientConfig(...),
    OKX: OkxDataClientConfig(...),
}

# 使用不同的 stream prefix
message_bus=MessageBusConfig(
    streams_prefix="crypto_data",  # 所有交易所数据合并到一个 stream
)
```

### 策略进程订阅特定数据

```python
# 只订阅某些交易所的数据
data_engine=LiveDataEngineConfig(
    external_clients=[
        ClientId("BINANCE"),  # 从 Redis 接收 Binance 数据
        # ClientId("BYBIT"),  # 不需要 Bybit 数据就不添加
    ],
)
```

## 故障处理

| 场景 | 数据采集进程 | 策略进程 | 结果 |
|------|-------------|---------|------|
| 策略崩溃/重启 | 继续运行 | 自动重连 Redis | ✅ 数据不丢失 |
| 数据采集崩溃 | 需要重启 | 等待数据恢复 | ⚠️ 数据中断 |
| Redis 重启 | 自动重连 | 自动重连 | ✅ 自动恢复 |
| 网络短暂中断 | 自动重连 | 使用缓冲区数据 | ✅ 基本不丢失 |

## 依赖安装

```bash
pip install -r requirements.txt
```

## 注意事项

1. 确保 Redis 版本 >= 6.2（支持 Streams 功能）
2. 两个进程的 `encoding` 参数必须一致
3. 生产者进程的 `streams_prefix` 必须匹配消费者进程的 `external_streams`
4. 建议在测试网环境先进行测试
