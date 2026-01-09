#!/bin/bash

# 配置项
RPC_URL="http://localhost:8545"
# Foundry/Anvil/Reth-Dev 默认的第一个私钥
PRIV_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"

while true; do
    echo "------------------------------------------"
    echo "$(date +'%H:%M:%S') - 开始发送 10 笔交易..."
    
    for i in {1..10}
    do
        # 生成一个随机地址
        RANDOM_ADDR="0x$(openssl rand -hex 20)"
        
        # 发送交易 (不等待确认以提高速度 --async)
        cast send $RANDOM_ADDR --value 0.01ether --private-key $PRIV_KEY --rpc-url $RPC_URL --async > /dev/null 2>&1
        
        echo -n "."
    done
    
    echo ""
    CURRENT_BLOCK=$(cast block-number --rpc-url $RPC_URL)
    echo "发送完成！当前最新区块高度: $CURRENT_BLOCK"
    echo "等待 30 秒进行下一轮..."
    
    sleep 30
done
