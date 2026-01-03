#[cfg(test)]
mod test_fn {

    #[test]
    fn test_1() {
        let name = "Rust".to_string();

        // 这个闭包内部把 name 吃掉了，就是 move 走了
        // 编译器推导它实现了 FnOnce
        let eat_name = || {
            drop(name);
        };

        // 第一次调用，成功，name 被丢弃
        eat_name();

        // eat_name(); // 报错，闭包已经被消耗了，不能再调用

        let name = "Python".to_string();
        let eat_name_box: Box<dyn FnOnce()> = Box::new(move || {
            println!("Eating {}", name);
            drop(name);
        });

        eat_name_box();
    }

    #[allow(dead_code)]
    fn run_once<F>(action: F)
    where
        F: FnOnce(),
    {
        action();

        // action();// 在函数里也不能调用两次
    }

    #[test]
    fn test_2() {
        let mut count = 0;

        // 闭包时修改了 count，所以它是 FnMut
        // 注意，变量 inc_counter 必须是 mut 的，因为调用它会改变它的内部状态
        let mut inc_counter = || {
            count += 1;
            println!("Count: {count}");
        };

        inc_counter(); // count = 1;
        inc_counter(); // count = 2;
    }

    #[allow(dead_code)]
    fn run_twice<F>(mut action: F)
    where
        F: FnMut(),
    {
        action();
        action();
    }
}

#[cfg(test)]
mod test_self_ref {
    use std::{cell::RefCell, rc::Rc};

    #[allow(dead_code)]
    struct Node {
        next: Option<Rc<RefCell<Node>>>, // 下一个节点
    }

    #[test]
    fn test_1() {
        // 创建节点 A
        // 此时 A 的引用计数 = 1，被变量 a 持有
        let a = Rc::new(RefCell::new(Node { next: None }));

        // 创建节点 b
        // 此时 b 的引用计数 = 1
        let b = Rc::new(RefCell::new(Node { next: None }));

        // 让 a 指向 b
        b.borrow_mut().next = Some(Rc::clone(&a));
    }

    #[allow(dead_code)]
    #[derive(Debug)]
    enum List {
        Cons(i32, RefCell<Rc<List>>),
        Nil,
    }

    #[allow(dead_code)]
    impl List {
        fn tail(&self) -> Option<&RefCell<Rc<List>>> {
            match self {
                List::Cons(_, item) => Some(item),
                List::Nil => None,
            }
        }
    }

    #[test]
    fn test_2() {
        let a = Rc::new(List::Cons(5, RefCell::new(Rc::new(List::Nil))));

        println!("a的初始化rc计数 = {}", Rc::strong_count(&a));
        println!("a指向的节点 = {:?}", a.tail());

        let b = Rc::new(List::Cons(10, RefCell::new(Rc::clone(&a))));
        println!("在b创建后，a的rc计数 = {}", Rc::strong_count(&a));
        println!("b的初始化rc计数 = {}", Rc::strong_count(&b));
        println!("b指向的节点 = {:?}", b.tail());

        if let Some(link) = a.tail() {
            *link.borrow_mut() = Rc::clone(&b);
        }

        println!("在更改a后，b的rc计数 = {}", Rc::strong_count(&b));
        println!("在更改a后，a的rc计数 = {}", Rc::strong_count(&a));

        // 下面一行println!将导致循环引用
        // 我们可怜的8MB大小的main线程栈空间将被它冲垮，最终造成栈溢出
        // println!("a next item = {:?}", a.tail());
    }

    #[test]
    fn test_3() {
        let five = Rc::new(5);

        let weak_five = Rc::downgrade(&five);

        let strong_five: Option<Rc<i32>> = weak_five.upgrade();

        assert_eq!(*strong_five.unwrap(), 5);

        drop(five);

        let strong_five: Option<Rc<i32>> = weak_five.upgrade();
        assert_eq!(strong_five, None);
    }
}

#[cfg(test)]
mod test_weak {
    use std::{
        cell::RefCell,
        rc::{Rc, Weak},
        vec,
    };

    #[allow(dead_code)]
    struct Owner {
        name: String,
        gadgets: RefCell<Vec<Weak<Gadget>>>,
    }

    #[allow(dead_code)]
    struct Gadget {
        id: i32,
        owner: Rc<Owner>,
    }

    #[test]
    fn test_0() {
        // 创建一个 Owner
        // 需要注意，该 Owner 也拥有多个 `gadgets`
        let gadget_owner: Rc<Owner> = Rc::new(Owner {
            name: "Gadget Man".to_string(),
            gadgets: RefCell::new(Vec::new()),
        });

        // 创建工具，同时与主人进行关联：创建两个 gadget，他们分别持有 gadget_owner 的一个引用。
        let gadget1 = Rc::new(Gadget {
            id: 1,
            owner: gadget_owner.clone(),
        });
        let gadget2 = Rc::new(Gadget {
            id: 2,
            owner: gadget_owner.clone(),
        });

        // 为主人更新它所拥有的工具
        // 因为之前使用了 `Rc`，现在必须要使用 `Weak`，否则就会循环引用
        gadget_owner
            .gadgets
            .borrow_mut()
            .push(Rc::downgrade(&gadget1));
        gadget_owner
            .gadgets
            .borrow_mut()
            .push(Rc::downgrade(&gadget2));

        // 遍历 gadget_owner 的 gadgets 字段
        for gadget_opt in gadget_owner.gadgets.borrow().iter() {
            // gadget_opt 是一个 Weak<Gadget> 。 因为 weak 指针不能保证他所引用的对象
            // 仍然存在。所以我们需要显式的调用 upgrade() 来通过其返回值(Option<_>)来判
            // 断其所指向的对象是否存在。
            // 当然，Option 为 None 的时候这个引用原对象就不存在了。
            let gadget = gadget_opt.upgrade().unwrap();
            println!("Gadget {} owned by {}", gadget.id, gadget.owner.name);
        }

        // 在 main 函数的最后，gadget_owner，gadget1 和 gadget2 都被销毁。
        // 具体是，因为这几个结构体之间没有了强引用（`Rc<T>`），所以，当他们销毁的时候。
        // 首先 gadget2 和 gadget1 被销毁。
        // 然后因为 gadget_owner 的引用数量为 0，所以这个对象可以被销毁了。
        // 循环引用问题也就避免了
    }

    #[allow(dead_code)]
    #[derive(Debug)]
    struct Node {
        value: i32,
        parent: RefCell<Weak<Node>>,
        children: RefCell<Vec<Rc<Node>>>,
    }

    #[test]
    fn test_1() {
        let leaf = Rc::new(Node {
            value: 3,
            parent: RefCell::new(Weak::new()),
            children: RefCell::new(vec![]),
        });

        println!(
            "leaf strong = {}, weak = {}",
            Rc::strong_count(&leaf),
            Rc::weak_count(&leaf),
        );

        {
            let branch = Rc::new(Node {
                value: 5,
                parent: RefCell::new(Weak::new()),
                children: RefCell::new(vec![Rc::clone(&leaf)]),
            });

            *leaf.parent.borrow_mut() = Rc::downgrade(&branch);

            println!(
                "branch strong = {}, weak = {}",
                Rc::strong_count(&branch),
                Rc::weak_count(&branch),
            );

            println!(
                "leaf strong = {}, weak = {}",
                Rc::strong_count(&leaf),
                Rc::weak_count(&leaf),
            );
        }

        println!("leaf parent = {:?}", leaf.parent.borrow().upgrade());
        println!(
            "leaf strong = {}, weak = {}",
            Rc::strong_count(&leaf),
            Rc::weak_count(&leaf),
        );
    }
}

#[allow(unused_imports)]
mod test_modify_week {
    use std::{
        cell::RefCell,
        rc::{Rc, Weak},
    };

    #[allow(dead_code)]
    struct Node {
        value: i32,
        // 父节点，使用weak 防止循环引用
        // 使用 RefCell 为了能修改父节点的值
        parent: RefCell<Weak<RefCell<Node>>>,
    }

    #[test]
    fn test_modify_weak() {
        let parent = Rc::new(RefCell::new(Node {
            value: 10,
            parent: RefCell::new(Weak::new()),
        }));

        let child = Rc::new(RefCell::new(Node {
            value: 20,
            parent: RefCell::new(Rc::downgrade(&parent)),
        }));

        println!("修改前父节点的值： {}", parent.borrow().value);

        // 子节点尝试修改父节点的值
        let weak_parent = child.borrow().parent.borrow().clone();

        if let Some(parent_rc) = weak_parent.upgrade() {
            parent_rc.borrow_mut().value = 999;
            println!("子节点成功修改了父节点的值");
        } else {
            println!("父节点已经不存在了");
        }

        println!("修改后父节点的值：{}", parent.borrow().value);
    }
}

#[cfg(test)]
mod test_pin {
    use std::marker::PhantomPinned;

    #[test]
    fn test_1() {
        println!("====== 1. 基本类型 stack ======");
        let mut x = 100;
        let mut y = 200;

        println!("交换前 x={}, y={}", x, y);
        //  场景：简单的整数交换
        std::mem::swap(&mut x, &mut y);
        println!("交换后 x={}, y={}", x, y);

        // 场景：用 -1 替换 x ，并拿走 x 原来的值
        let old_x = std::mem::replace(&mut x, -1);
        println!("Replace 后， x={}, old_x={}", x, old_x);

        println!("====== 2. 堆上数据 ======");
        let mut s1 = "I am A".to_string();
        let mut s2 = "I am B".to_string();

        println!("交换前， s1={}, s2={}", s1, s2);

        // 这里仅仅交换了栈上的胖指针， ptr len cap
        // 堆上的实际字节数组并没有被复制或者移动，所以速度极快
        std::mem::swap(&mut s1, &mut s2);
        println!("交换后， s1={}, s2={}", s1, s2);

        let mut v = vec![1, 2, 3];
        // 场景：拿走 vector 的所有权，留下一个空的 vector
        let old_v = std::mem::take(&mut v);

        println!(
            "Take后: v长度={:?} (被掏空), old_v={:?} (拿到手)",
            v.len(),
            old_v
        );
    }

    #[allow(dead_code)]
    struct LockedVec {
        data: Vec<i32>,

        // 这个标记就像给结构体打了禁止移动的钢印
        _pin: PhantomPinned,
    }

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    #[test]
    fn test_2() {
        let v = LockedVec {
            data: vec![1, 2, 3],
            _pin: PhantomPinned,
        };

        let mut pinned_wrapper = Box::pin(v);

        let mut other_wrapper = Box::pin(LockedVec {
            data: vec![4, 5, 6],
            _pin: PhantomPinned,
        });

        // std::mem::swap(pinned_wrapper.as_mut(), other_wrapper.as_mut());
    }
}
