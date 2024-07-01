use core::future::Future;

pub trait RobotBehavior {
    fn spawn_task<F>(task: F)
    where
        F: FnOnce() -> dyn Future<Output = ()>;

    fn get_distance_sensor() -> impl Future<Output = ()> + Send;
}

// struct Robot {}
//
// impl RobotBehavior for Robot {
//     fn get_distance_sensor() -> impl Future<Output = ()> + Send {
//         core::future::ready(())
//     }
//
//     fn spawn_task<F>(task: F)
//     where
//         F: FnOnce() -> dyn Future<Output = ()>,
//     {
//         todo!()
//     }
// }
