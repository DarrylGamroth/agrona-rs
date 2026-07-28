// Copyright 2014-2025 Real Logic Limited.
// Copyright 2026 Rubus Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Release/acquire counter publication acceptance on native CI architectures.

mod support;

use std::hint::spin_loop;
use std::sync::atomic::Ordering;
use std::thread;

use agrona::concurrent::status::CountersReader;
use support::SharedAlignedBuffer;

#[test]
fn allocated_state_release_publishes_each_metadata_record() {
    const RECORDS: usize = 4_096;
    let metadata = SharedAlignedBuffer::new(RECORDS * CountersReader::METADATA_LENGTH);
    let values = SharedAlignedBuffer::new(RECORDS * CountersReader::COUNTER_LENGTH);
    let reader = CountersReader::new(metadata.as_bytes(), values.as_bytes()).unwrap();

    thread::scope(|scope| {
        let reader_thread = scope.spawn(move || {
            for counter_id in 0..RECORDS as i32 {
                while reader.counter_state(counter_id).unwrap() != CountersReader::RECORD_ALLOCATED
                {
                    spin_loop();
                }

                let expected = counter_id as i64 + 1;
                assert_eq!(
                    counter_id.wrapping_mul(17),
                    reader.counter_type_id(counter_id).unwrap()
                );
                assert_eq!(
                    expected,
                    reader.free_for_reuse_deadline(counter_id).unwrap()
                );
                assert_eq!(expected, reader.counter_value(counter_id).unwrap());
                assert_eq!(
                    expected.wrapping_mul(3),
                    reader.counter_registration_id(counter_id).unwrap()
                );
                assert_eq!(expected as u8, reader.counter_key(counter_id).unwrap()[0]);
                assert_eq!(
                    expected.to_ne_bytes(),
                    reader.counter_label(counter_id).unwrap()
                );
            }
        });

        for counter_id in 0..RECORDS as i32 {
            let metadata_offset = CountersReader::metadata_offset(counter_id).unwrap();
            let values_offset = CountersReader::counter_offset(counter_id).unwrap();
            let value = counter_id as i64 + 1;

            metadata.store_i32(
                metadata_offset + CountersReader::TYPE_ID_OFFSET,
                counter_id.wrapping_mul(17),
                Ordering::Relaxed,
            );
            metadata.store_i64(
                metadata_offset + CountersReader::FREE_FOR_REUSE_DEADLINE_OFFSET,
                value,
                Ordering::Relaxed,
            );
            metadata.write_bytes(metadata_offset + CountersReader::KEY_OFFSET, &[value as u8]);
            metadata.write_bytes(
                metadata_offset + CountersReader::LABEL_VALUE_OFFSET,
                &value.to_ne_bytes(),
            );
            metadata.store_i32(
                metadata_offset + CountersReader::LABEL_LENGTH_OFFSET,
                size_of::<i64>() as i32,
                Ordering::Release,
            );

            values.store_i64(
                values_offset + CountersReader::COUNTER_VALUE_OFFSET,
                value,
                Ordering::Release,
            );
            values.store_i64(
                values_offset + CountersReader::REGISTRATION_ID_OFFSET,
                value.wrapping_mul(3),
                Ordering::Release,
            );
            metadata.store_i32(
                metadata_offset + CountersReader::STATE_OFFSET,
                CountersReader::RECORD_ALLOCATED,
                Ordering::Release,
            );
        }

        reader_thread.join().unwrap();
    });
}
