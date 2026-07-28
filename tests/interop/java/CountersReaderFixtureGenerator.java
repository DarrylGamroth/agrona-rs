/*
 * Copyright 2014-2025 Real Logic Limited.
 * Copyright 2026 Rubus Technologies Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.file.Files;
import java.nio.file.Path;

import org.agrona.concurrent.UnsafeBuffer;
import org.agrona.concurrent.status.CountersManager;
import org.agrona.concurrent.status.CountersReader;

import static java.nio.charset.StandardCharsets.US_ASCII;

public final class CountersReaderFixtureGenerator
{
    private static final int CAPACITY = 4;

    private CountersReaderFixtureGenerator()
    {
    }

    public static void main(final String[] arguments) throws IOException
    {
        if (1 != arguments.length)
        {
            throw new IllegalArgumentException("expected output directory");
        }

        final ByteBuffer metadataBytes =
            ByteBuffer.allocateDirect(CAPACITY * CountersReader.METADATA_LENGTH);
        final ByteBuffer valuesBytes =
            ByteBuffer.allocateDirect(CAPACITY * CountersReader.COUNTER_LENGTH);
        final UnsafeBuffer metadata = new UnsafeBuffer(metadataBytes);
        final UnsafeBuffer values = new UnsafeBuffer(valuesBytes);
        final CountersManager manager =
            new CountersManager(metadata, values, US_ASCII, () -> 1_234L, 55L);

        final int first = manager.allocate(
            "alpha",
            7,
            (key) ->
            {
                for (int index = 0; index < CountersReader.MAX_KEY_LENGTH; index++)
                {
                    key.putByte(index, (byte)index);
                }
            });
        manager.setCounterValue(first, 42L);
        manager.setCounterRegistrationId(first, 1_001L);
        manager.setCounterOwnerId(first, 2_002L);
        manager.setCounterReferenceId(first, 3_003L);

        final int second = manager.allocate(
            "x".repeat(CountersReader.MAX_LABEL_LENGTH),
            -5,
            (key) -> key.setMemory(0, CountersReader.MAX_KEY_LENGTH, (byte)0xA5));
        manager.setCounterValue(second, -42L);
        manager.setCounterRegistrationId(second, -1_001L);
        manager.setCounterOwnerId(second, -2_002L);
        manager.setCounterReferenceId(second, -3_003L);

        final int reclaimed = manager.allocate(
            "reclaimed",
            99,
            (key) -> key.setMemory(0, CountersReader.MAX_KEY_LENGTH, (byte)0x5A));
        manager.setCounterValue(reclaimed, 77L);
        manager.setCounterRegistrationId(reclaimed, 88L);
        manager.free(reclaimed);

        final Path output = Path.of(arguments[0]);
        Files.createDirectories(output);
        Files.write(output.resolve("metadata.bin"), copy(metadataBytes));
        Files.write(output.resolve("values.bin"), copy(valuesBytes));
    }

    private static byte[] copy(final ByteBuffer source)
    {
        final ByteBuffer duplicate = source.duplicate();
        duplicate.clear();
        final byte[] bytes = new byte[duplicate.remaining()];
        duplicate.get(bytes);
        return bytes;
    }
}

