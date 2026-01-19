use rocksdb::WriteBatch;

#[test]
fn test_rocketdb_crud() -> anyhow::Result<(), Box<dyn std::error::Error>> {
    let path = "myeb_ex1";
    let db = rocksdb::DB::open_default(path)?;

    // 1. Put
    db.put(b"1", b"Alice")?;
    db.put(b"2", b"Bob")?;
    db.put(b"3", b"Charlie")?;

    // 2. Get
    match db.get(b"1")? {
        Some(value) => println!("Found: {}", String::from_utf8(value)?),
        None => println!("Not found"),
    }

    // 3. delete
    db.delete(b"2")?;

    // 4. verify delete
    if db.get(b"2")?.is_none() {
        println!("Delete successful");
    } else {
        println!("Delete failed");
    }

    Ok(())
}

#[test]
fn test_write_batch() -> anyhow::Result<()> {
    let db = rocksdb::DB::open_default("mydb_ex2")?;

    // 初始化
    db.put(b"A", b"100")?;
    db.put(b"B", b"0")?;

    // 开始转账原子操作
    let mut batch = WriteBatch::default();

    // 从 A 扣款 50
    batch.put(b"A", b"50");
    // 给 B 加款 50
    batch.put(b"B", b"50");

    // 提交批处理
    db.write(batch)?;

    // 验证结果
    let a = String::from_utf8(db.get(b"A")?.unwrap())?;
    let b = String::from_utf8(db.get(b"B")?.unwrap())?;
    println!("A balance: {}, B balance: {}", a, b);

    Ok(())
}

#[test]
fn test_scan() -> anyhow::Result<()> {
    let db = rocksdb::DB::open_default("mydb_ex3")?;

    // 插入一些数据
    db.put(b"2023:01:001", b"Log A")?;
    db.put(b"2023:01:002", b"Log B")?;
    db.put(b"2023:02:001", b"Log C")?;
    db.put(b"2024:01:001", b"Log D")?;

    let prefix = b"2023:01";
    println!("Scanning for prefix: {:?}", String::from_utf8_lossy(prefix));

    // 从前缀开始迭代
    let iter = db.iterator(rocksdb::IteratorMode::From(
        prefix,
        rocksdb::Direction::Forward,
    ));

    for item in iter {
        let (key, value) = item?;

        // 必须手动检查  key 是否以 prefix 前缀开头
        // 因为 RocksDB 的迭代器会一直往后走到 2024年去
        if !key.starts_with(prefix) {
            break;
        }
        println!(
            "Found key: {}, value: {}",
            String::from_utf8_lossy(&key),
            String::from_utf8_lossy(&value)
        );
    }

    Ok(())
}
