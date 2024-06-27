use index_queue::IndexQueue;

#[derive(Clone, Debug)]
pub struct Collapser {
    orders: Vec<IndexQueue>,
    num_orders: usize,
}

impl Collapser {
    pub fn new() -> Self {
        Self {
            orders: vec![],
            num_orders: 0,
        }
    }

    pub fn push_order(&mut self, order: usize, cache_size: usize) {
        while self.orders.len() <= cache_size {
            self.orders.push(IndexQueue::default());
        }

        if !self.orders[cache_size].contains(order) {
            self.num_orders += 1;
        }

        self.orders[cache_size].push_back(order);
    }

    pub fn remove_order(&mut self, order: usize, cache_size: usize) {
        if self.orders.len() <= cache_size {
            return;
        }

        self.orders[cache_size].remove(order);
        self.num_orders -= 1;
    }

    pub fn pop_order(&mut self) -> usize {
        for queue in self.orders.iter_mut() {
            if !queue.is_empty() {
                self.num_orders -= 1;
                return queue.pop_front().unwrap();
            }
        }

        return 0;
    }

    pub fn is_empty(&self) -> bool {
        self.num_orders == 0
    }
}
