// pub enum BevyQueryScope {
//     Entity(Entity),
//     World
// }

// pub struct BevyQuery<T: Component + Clone> {
//     /// Filter and components being checked for
//     content: T,
// }

// pub fn changed<T: Component + Clone>(
//     query: Query<Ref<T>>,
// ) {
//     let sample = BevyQueryScope::World;

//     match sample {
//         BevyQueryScope::Entity(entity) => {
//             let Ok(data) =query.get(entity)
//             .inspect_err(|err| warn!("blah blah blah thing doesn't exist")) else {
//                 todo!("something somehting something, tell dioxus that entity is invalid and to requery for entity based on component or something");
//                 (entity, data);
//                 return;
//             }
//         },
//         BevyQueryScope::World => {f
//             let datas = query.iter().clone();
//                 todo!("send this to dioxus for it to read inside BevyQuery or somethign and update its state with this component")
//                 datas

//             },
//     }
// }
