use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::cell::RefCell;

pub use nes_derive::SaveState;

pub trait SaveState {
    type Data: Serialize + DeserializeOwned;

    fn save_state(&self) -> Self::Data;
    fn restore_state(&mut self, state: &Self::Data);
}

pub struct OptionData<T: SaveState>(Option<<T as SaveState>::Data>);

impl<T: SaveState> Clone for OptionData<T>
where
    T::Data: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: SaveState> Serialize for OptionData<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, T: SaveState> Deserialize<'de> for OptionData<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Option::deserialize(deserializer).map(OptionData)
    }
}

impl<T: SaveState> SaveState for Option<T> {
    type Data = OptionData<T>;

    fn save_state(&self) -> Self::Data {
        OptionData(self.as_ref().map(|s| s.save_state()))
    }

    fn restore_state(&mut self, state: &Self::Data) {
        if let Some((item, state)) = self.as_mut().zip(state.0.as_ref()) {
            item.restore_state(state);
        }
    }
}

pub struct VecData<T: SaveState>(Vec<<T as SaveState>::Data>);

impl<T: SaveState> Serialize for VecData<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, T: SaveState> Deserialize<'de> for VecData<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Vec::deserialize(deserializer).map(VecData)
    }
}

impl<T: SaveState> SaveState for Vec<T> {
    type Data = VecData<T>;

    fn save_state(&self) -> Self::Data {
        VecData(self.iter().map(|v| v.save_state()).collect())
    }

    fn restore_state(&mut self, state: &Self::Data) {
        for (v, state) in self.iter_mut().zip(state.0.iter()) {
            v.restore_state(state);
        }
    }
}

impl<const N: usize, T: SaveState> Serialize for Arr<N, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

pub struct Arr<const N: usize, T: SaveState>([T::Data; N]);

macro_rules! impl_arr_save_state {
    ($($n:literal) +) => {
        $(


            impl<'de, T: SaveState> Deserialize<'de> for Arr<$n, T> {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    <[T::Data; $n]>::deserialize(deserializer).map(Arr)
                }
            }

            impl<T: SaveState> SaveState for [T; $n] {
                type Data = Arr<$n, T>;

                fn save_state(&self) -> Self::Data {
                    let mut arr = [const { std::mem::MaybeUninit::uninit() }; $n];
                    for (s, v) in self.iter().zip(arr.iter_mut()) {
                        *v = std::mem::MaybeUninit::new(s.save_state());
                    }
                    let arr = arr.map(|v| unsafe { v.assume_init() });
                    Arr(arr)
                }

                fn restore_state(&mut self, state: &Self::Data) {
                    for (v, state) in self.iter_mut().zip(state.0.iter()) {
                        v.restore_state(state);
                    }
                }
            }

        )+
    };
}

impl_arr_save_state!(1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 32);

impl<T: SaveState> SaveState for RefCell<T> {
    type Data = T::Data;

    fn save_state(&self) -> Self::Data {
        self.borrow().save_state()
    }

    fn restore_state(&mut self, state: &Self::Data) {
        self.borrow_mut().restore_state(state)
    }
}

pub trait BinarySaveState {
    fn binary_save_state(&self) -> Vec<u8>;
    fn binary_restore_state(&mut self, state: &[u8]);
}

impl<T: SaveState> BinarySaveState for T {
    fn binary_save_state(&self) -> Vec<u8> {
        let data = self.save_state();
        postcard::to_allocvec(&data).unwrap()
    }

    fn binary_restore_state(&mut self, state: &[u8]) {
        let data = postcard::from_bytes::<T::Data>(state).unwrap();
        self.restore_state(&data);
    }
}
