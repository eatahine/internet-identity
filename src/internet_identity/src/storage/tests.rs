use crate::activity_stats::activity_counter::active_anchor_counter::ActiveAnchorCounter;
use crate::activity_stats::{ActivityStats, CompletedActivityStats, OngoingActivityStats};
use crate::archive::{ArchiveData, ArchiveState};
use crate::state::PersistentState;
use crate::storage::anchor::{Anchor, Device, KeyTypeInternal};
use crate::storage::{Header, PersistentStateError, StorageError};
use crate::Storage;
use candid::Principal;
use ic_stable_structures::{Memory, VectorMemory};
use internet_identity_interface::internet_identity::types::{
    ArchiveConfig, DeviceProtection, Purpose,
};
use serde_bytes::ByteBuf;
use std::rc::Rc;

const WASM_PAGE_SIZE: u64 = 1 << 16;
const HEADER_SIZE: usize = 66;
const RESERVED_HEADER_BYTES: u64 = 2 * WASM_PAGE_SIZE;
const LENGTH_OFFSET: u64 = 2;
const PERSISTENT_STATE_MAGIC: [u8; 4] = *b"IIPS";

#[test]
fn should_match_actual_header_size() {
    // if this test fails, make sure the change was intentional and upgrade as well as rollback still work!
    assert_eq!(std::mem::size_of::<Header>(), HEADER_SIZE);
}

#[test]
fn should_report_max_number_of_entries_for_32gb() {
    let memory = VectorMemory::default();
    let storage = Storage::new((1, 2), memory);
    assert_eq!(storage.max_entries(), 8_178_860);
}

#[test]
fn should_serialize_header_v7() {
    let memory = VectorMemory::default();
    let mut storage = Storage::new((1, 2), memory.clone());
    storage.update_salt([5u8; 32]);
    storage.flush();

    let mut buf = vec![0; HEADER_SIZE];
    memory.read(0, &mut buf);
    assert_eq!(buf, hex::decode("494943070000000001000000000000000200000000000000001005050505050505050505050505050505050505050505050505050505050505050000020000000000").unwrap());
}

#[test]
fn should_recover_header_from_memory_v7() {
    let memory = VectorMemory::default();
    memory.grow(1);
    memory.write(0, &hex::decode("494943070500000040e2010000000000f1fb090000000000000843434343434343434343434343434343434343434343434343434343434343430002000000000000000000000000000000000000000000000000").unwrap());

    let storage = Storage::from_memory(memory).unwrap();
    assert_eq!(storage.assigned_anchor_number_range(), (123456, 654321));
    assert_eq!(storage.salt().unwrap(), &[67u8; 32]);
    assert_eq!(storage.anchor_count(), 5);
    assert_eq!(storage.version(), 7);
}

fn add_test_anchor_data<M: Memory + Clone>(storage: &mut Storage<M>, number_of_anchors: usize) {
    for _ in 0..number_of_anchors {
        let (anchor_number, mut anchor) = storage
            .allocate_anchor()
            .expect("Failure allocating an anchor.");
        anchor
            .add_device(sample_unique_device(anchor_number as usize))
            .expect("Failure adding a device");
        storage.write(anchor_number, anchor.clone()).unwrap();
    }
}

#[test]
fn should_allocate_new_bucket_after_2048_anchors_v7() {
    let (id_range_lo, id_range_hi) = (12345, 678910);
    let memory_v7 = VectorMemory::default();
    let mut storage_v7 = Storage::new((id_range_lo, id_range_hi), memory_v7.clone());

    // The 1st anchor allocates 1st bucket.
    add_test_anchor_data(&mut storage_v7, 1);
    assert_eq!(130, memory_v7.size()); // 2 header pages plus 1st bucket of 128 pages.

    // With a total of 2048 anchors, we still have only one bucket.
    add_test_anchor_data(&mut storage_v7, 2047);
    assert_eq!(130, memory_v7.size()); // 2 header pages plus 1st bucket of 128 pages.

    // For the next anchor a new bucket of 128 pages will be allocated.
    add_test_anchor_data(&mut storage_v7, 1);
    assert_eq!(258, memory_v7.size()); // 2 header pages plus two buckets of 128 pages each.
}

#[test]
fn should_read_previous_write() {
    let memory = VectorMemory::default();
    let mut storage = Storage::new((12345, 678910), memory);
    let (anchor_number, mut anchor) = storage.allocate_anchor().unwrap();

    anchor.add_device(sample_device()).unwrap();
    storage.write(anchor_number, anchor.clone()).unwrap();

    let read_anchor = storage.read(anchor_number).unwrap();
    assert_eq!(anchor, read_anchor);
}

#[test]
fn should_serialize_first_record() {
    let memory = VectorMemory::default();
    let mut storage = Storage::new((123, 456), memory.clone());
    let (anchor_number, mut anchor) = storage.allocate_anchor().unwrap();
    assert_eq!(anchor_number, 123u64);

    anchor.add_device(sample_device()).unwrap();
    let expected_length = candid::encode_one(&anchor).unwrap().len();

    storage.write(anchor_number, anchor.clone()).unwrap();

    let mut buf = vec![0u8; expected_length];
    memory.read(RESERVED_HEADER_BYTES + LENGTH_OFFSET, &mut buf);
    let decoded_from_memory: Anchor = candid::decode_one(&buf).unwrap();
    assert_eq!(decoded_from_memory, anchor);
}

#[test]
fn should_serialize_subsequent_record_to_expected_memory_location() {
    const EXPECTED_RECORD_OFFSET: u64 = 409_600; // 100 * max anchor size
    let memory = VectorMemory::default();
    let mut storage = Storage::new((123, 456), memory.clone());
    for _ in 0..100 {
        storage.allocate_anchor().unwrap();
    }
    let (anchor_number, mut anchor) = storage.allocate_anchor().unwrap();
    assert_eq!(anchor_number, 223u64);

    anchor.add_device(sample_device()).unwrap();
    let expected_length = candid::encode_one(&anchor).unwrap().len();

    storage.write(anchor_number, anchor.clone()).unwrap();

    let mut buf = vec![0u8; expected_length];
    memory.read(
        RESERVED_HEADER_BYTES + EXPECTED_RECORD_OFFSET + LENGTH_OFFSET,
        &mut buf,
    );
    let decoded_from_memory: Anchor = candid::decode_one(&buf).unwrap();
    assert_eq!(decoded_from_memory, anchor);
}

#[test]
fn should_not_write_using_anchor_number_outside_allocated_range() {
    let memory = VectorMemory::default();
    let mut storage = Storage::new((123, 456), memory);
    let (_, anchor) = storage.allocate_anchor().unwrap();

    let result = storage.write(222, anchor);
    assert!(matches!(result, Err(StorageError::BadAnchorNumber(_))))
}

#[test]
fn should_deserialize_first_record() {
    let memory = VectorMemory::default();
    memory.grow(3);
    let mut storage = Storage::new((123, 456), memory.clone());
    let (anchor_number, mut anchor) = storage
        .allocate_anchor()
        .expect("Failed to allocate an anchor");
    storage
        .write(anchor_number, anchor.clone())
        .expect("Failed to write anchor");
    assert_eq!(anchor_number, 123u64);

    anchor.add_device(sample_device()).unwrap();
    let buf = candid::encode_one(&anchor).unwrap();
    memory.write(RESERVED_HEADER_BYTES, &(buf.len() as u16).to_le_bytes());
    memory.write(RESERVED_HEADER_BYTES + 2, &buf);

    let read_from_storage = storage.read(123).unwrap();
    assert_eq!(read_from_storage, anchor);
}

#[test]
fn should_deserialize_subsequent_record_at_expected_memory_location() {
    const EXPECTED_RECORD_OFFSET: u64 = 409_600; // 100 * max anchor size
    let memory = VectorMemory::default();
    memory.grow(9); // grow memory to accommodate a write to record 100
    let mut storage = Storage::new((123, 456), memory.clone());
    for _ in 0..100 {
        let (anchor_number, anchor) = storage
            .allocate_anchor()
            .expect("Failed to allocate an anchor");
        storage
            .write(anchor_number, anchor)
            .expect("Failed to write anchor");
    }
    let (anchor_number, mut anchor) = storage.allocate_anchor().unwrap();
    assert_eq!(anchor_number, 223u64);

    anchor.add_device(sample_device()).unwrap();
    let buf = candid::encode_one(&anchor).unwrap();
    memory.write(
        RESERVED_HEADER_BYTES + EXPECTED_RECORD_OFFSET,
        &(buf.len() as u16).to_le_bytes(),
    );
    memory.write(RESERVED_HEADER_BYTES + 2 + EXPECTED_RECORD_OFFSET, &buf);

    let read_from_storage = storage.read(223).unwrap();
    assert_eq!(read_from_storage, anchor);
}

#[test]
fn should_not_read_using_anchor_number_outside_allocated_range() {
    let memory = VectorMemory::default();
    let mut storage = Storage::new((123, 456), memory);
    storage.allocate_anchor().unwrap();

    let result = storage.read(222);
    assert!(matches!(result, Err(StorageError::BadAnchorNumber(_))))
}

#[test]
fn should_save_and_restore_persistent_state() {
    let memory = VectorMemory::default();
    let mut storage = Storage::new((123, 456), memory);
    storage.flush();
    storage.allocate_anchor().unwrap();

    let persistent_state = sample_persistent_state();

    storage.write_persistent_state(&persistent_state);
    assert_eq!(storage.read_persistent_state().unwrap(), persistent_state);
}

#[test]
fn should_save_persistent_state_at_expected_memory_address() {
    let memory = VectorMemory::default();
    let mut storage = Storage::new((10_000, 3_784_873), memory.clone());
    storage.flush();

    storage.write_persistent_state(&sample_persistent_state());

    let mut buf = vec![0u8; 4];
    memory.read(RESERVED_HEADER_BYTES, &mut buf);
    assert_eq!(buf, PERSISTENT_STATE_MAGIC);
}

#[test]
fn should_not_find_persistent_state() {
    let memory = VectorMemory::default();
    let mut storage = Storage::new((10_000, 3_784_873), memory);
    storage.flush();

    let result = storage.read_persistent_state();
    assert!(matches!(result, Err(PersistentStateError::NotFound)))
}

#[test]
fn should_not_find_persistent_state_on_magic_bytes_mismatch() {
    let memory = VectorMemory::default();
    memory.grow(3);

    let mut storage = Storage::new((10_000, 3_784_873), memory.clone());
    storage.flush();

    memory.write(RESERVED_HEADER_BYTES, b"IIPX"); // correct magic bytes are IIPS

    let result = storage.read_persistent_state();
    assert!(matches!(result, Err(PersistentStateError::NotFound)))
}

#[test]
fn should_save_persistent_state_at_expected_memory_address_with_anchors() {
    const EXPECTED_ADDRESS: u64 = RESERVED_HEADER_BYTES + 100 * 4096; // number of anchors is 100

    let memory = VectorMemory::default();
    let mut storage = Storage::new((10_000, 3_784_873), memory.clone());
    storage.flush();

    for _ in 0..100 {
        storage.allocate_anchor().unwrap();
    }

    storage.write_persistent_state(&sample_persistent_state());

    let mut buf = vec![0u8; 4];
    memory.read(EXPECTED_ADDRESS, &mut buf);
    assert_eq!(buf, PERSISTENT_STATE_MAGIC);
}

/// This tests verifies that address calculation is correct for 64bit addresses.
/// Note: this test takes about 8GB of memory.
#[test]
fn should_save_persistent_state_at_expected_memory_address_with_many_anchors() {
    let memory = VectorMemory::default();
    memory.grow(1);
    memory.write(0, &hex::decode("4949430760E316001027000000000000a9c03900000000000010434343434343434343434343434343434343434343434343434343434343434300000200").unwrap());
    const EXPECTED_ADDRESS: u64 = RESERVED_HEADER_BYTES + 1_500_000 * 4096; // number of anchors is 1_500_000

    let mut storage = Storage::from_memory(memory.clone()).unwrap();
    storage.write_persistent_state(&sample_persistent_state());

    let mut buf = vec![0u8; 4];
    memory.read(EXPECTED_ADDRESS, &mut buf);
    assert_eq!(buf, PERSISTENT_STATE_MAGIC);
}

/// This test verifies that storage correctly reports `NotFound` if the persistent state address
/// lies outside of the allocated stable memory range. This can happen on upgrade from a version
/// that did not serialize a persistent state into stable memory.
#[test]
fn should_not_panic_on_unallocated_persistent_state_mem_address() {
    let memory = VectorMemory::default();
    let mut storage = Storage::new((10_000, 3_784_873), memory);
    storage.flush();
    for _ in 0..32 {
        storage.allocate_anchor();
    }

    assert!(matches!(
        storage.read_persistent_state(),
        Err(PersistentStateError::NotFound)
    ));
}

#[test]
fn should_overwrite_persistent_state_with_next_anchor() {
    const EXPECTED_ADDRESS: u64 = RESERVED_HEADER_BYTES + 4096; // only one anchor exists

    let memory = VectorMemory::default();
    let mut storage = Storage::new((10_000, 3_784_873), memory.clone());
    storage.flush();

    storage.allocate_anchor().unwrap();
    storage.write_persistent_state(&sample_persistent_state());

    let mut buf = vec![0u8; 4];
    memory.read(EXPECTED_ADDRESS, &mut buf);
    assert_eq!(buf, PERSISTENT_STATE_MAGIC);

    let (anchor_number, anchor) = storage.allocate_anchor().unwrap();
    storage.write(anchor_number, anchor).unwrap();

    let mut buf = vec![0u8; 4];
    memory.read(EXPECTED_ADDRESS, &mut buf);
    assert_ne!(buf, PERSISTENT_STATE_MAGIC);
    let result = storage.read_persistent_state();
    println!("{result:?}");
    assert!(matches!(result, Err(PersistentStateError::NotFound)));
}

#[test]
fn should_read_previously_stored_persistent_state() {
    const NUM_ANCHORS: u64 = 3;
    const EXPECTED_ADDRESS: u64 = RESERVED_HEADER_BYTES + NUM_ANCHORS * 4096;
    const PERSISTENT_STATE_BYTES: &str = "4949505368010000000000004449444c116c03949d879d0701f7f5cbfb0778eed5f3af090a6b04dee7beb6080291a5fcf10a7fd1d3dab70b05c8bbeff50d066c01c2adc9be0c036c04c3f9fca002788beea8c5047881cfaef40a0487eb979d0d7a6d7b6c02d6a9bbae0a78c2adc9be0c036c02aaac8d930407c2adc9be0c036c03c7e8ccee037884fbf0820968cfd6ffea0f086d096c04c7e8ccee0378f2f099840704938da78c0a78d6a9bbae0a786e0b6c028bc3e2f9040cbbd492d8090f6c0297beb4cb080d8b858cea090d6e0e6c02fcdde6ea0178f9a6ebf703786c0297beb4cb08108b858cea090e6d0e0100032700000000000000010a00000000006000b0010100005847f80d0000001027000000000000206363636363636363636363636363636363636363636363636363636363636363e8038002e1df0200000001000163000000000000006dbb0e000000000001420000000000000030f1c520000000002c00000000000000d133b45001000000";
    let memory = VectorMemory::default();
    // allocate space for the writes
    memory.grow(3);

    // Create storage, and add anchors (so that MemoryManager allocates a memory block for anchors).
    let mut storage = Storage::new((1, 100), memory.clone());
    for _ in 0..NUM_ANCHORS {
        let (anchor_number, anchor) = storage.allocate_anchor().expect("Failed allocating anchor");
        storage
            .write(anchor_number, anchor)
            .expect("Failed writing anchor");
    }

    memory.write(
        EXPECTED_ADDRESS,
        &hex::decode(PERSISTENT_STATE_BYTES).unwrap(),
    );

    assert_eq!(
        storage
            .read_persistent_state()
            .expect("Failed to read persistent state"),
        sample_persistent_state()
    );
}

fn sample_unique_device(id: usize) -> Device {
    Device {
        alias: format!(" #{}", id),
        ..sample_device()
    }
}

fn sample_device() -> Device {
    Device {
        pubkey: ByteBuf::from("hello world, I am a public key"),
        alias: "my test device".to_string(),
        credential_id: Some(ByteBuf::from("this is the credential id")),
        purpose: Purpose::Authentication,
        key_type: KeyTypeInternal::Unknown,
        protection: DeviceProtection::Unprotected,
        origin: None,
        last_usage_timestamp: Some(1234),
        metadata: None,
    }
}

fn sample_persistent_state() -> PersistentState {
    PersistentState {
        archive_state: ArchiveState::Created {
            data: ArchiveData {
                sequence_number: 39,
                archive_canister: Principal::from_text("2h5ob-7aaaa-aaaad-aacya-cai").unwrap(),
                entries_buffer: Rc::new(vec![]),
            },
            config: ArchiveConfig {
                module_hash: [99u8; 32],
                entries_buffer_limit: 10_000,
                polling_interval_ns: 60_000_000_000,
                entries_fetch_limit: 1_000,
            },
        },
        canister_creation_cycles_cost: 12_346_000_000,
        registration_rate_limit: None,
        active_anchor_stats: Some(ActivityStats {
            completed: CompletedActivityStats {
                daily_events: Some(ActiveAnchorCounter {
                    start_timestamp: 965485,
                    counter: 99,
                }),
                monthly_events: None,
            },
            ongoing: OngoingActivityStats {
                daily_events: ActiveAnchorCounter {
                    start_timestamp: 5648954321,
                    counter: 44,
                },
                monthly_events: vec![ActiveAnchorCounter {
                    start_timestamp: 549843248,
                    counter: 66,
                }],
            },
        }),
        domain_active_anchor_stats: None,
        latest_delegation_origins: None,
        max_num_latest_delegation_origins: None,
    }
}
