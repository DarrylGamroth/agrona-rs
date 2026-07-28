/*
 * Copyright 2014-2025 Real Logic Limited.
 * Copyright 2026 Rubus Technologies Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.concurrent.atomic.AtomicLong;

import org.agrona.concurrent.UnsafeBuffer;
import org.agrona.concurrent.status.CountersManager;
import org.agrona.concurrent.status.CountersReader;

import static java.nio.charset.StandardCharsets.US_ASCII;

public final class CountersReaderFixtureGenerator
{
    private static final int CAPACITY = 5;

    private CountersReaderFixtureGenerator()
    {
    }

    public static void main(final String[] arguments) throws IOException
    {
        if (1 == arguments.length)
        {
            generate(Path.of(arguments[0]));
        }
        else if (2 == arguments.length && "validate".equals(arguments[0]))
        {
            validate(Path.of(arguments[1]));
        }
        else
        {
            throw new IllegalArgumentException("expected output directory or: validate input-directory");
        }
    }

    private static void generate(final Path output) throws IOException
    {
        final ByteBuffer metadataBytes =
            ByteBuffer.allocateDirect(CAPACITY * CountersReader.METADATA_LENGTH);
        final ByteBuffer valuesBytes =
            ByteBuffer.allocateDirect(CAPACITY * CountersReader.COUNTER_LENGTH);
        final UnsafeBuffer metadata = new UnsafeBuffer(metadataBytes);
        final UnsafeBuffer values = new UnsafeBuffer(valuesBytes);
        final AtomicLong nowMs = new AtomicLong(1_234L);
        final CountersManager manager =
            new CountersManager(metadata, values, US_ASCII, nowMs::get, 55L);

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

        final int reuseCandidate = manager.allocate(
            "old",
            99,
            (key) -> key.setMemory(0, CountersReader.MAX_KEY_LENGTH, (byte)0x5A));
        manager.setCounterValue(reuseCandidate, 77L);
        manager.setCounterRegistrationId(reuseCandidate, 88L);
        manager.setCounterOwnerId(reuseCandidate, 89L);
        manager.setCounterReferenceId(reuseCandidate, 90L);
        manager.free(reuseCandidate);
        nowMs.set(1_289L);
        final int reused = manager.allocate("reused", 55);
        require(reuseCandidate == reused, "Java fixture reuse");

        final int reclaimed = manager.allocate(
            "reclaimed",
            99,
            (key) -> key.setMemory(0, CountersReader.MAX_KEY_LENGTH, (byte)0x5A));
        manager.setCounterValue(reclaimed, 177L);
        manager.setCounterRegistrationId(reclaimed, 188L);
        manager.free(reclaimed);

        Files.createDirectories(output);
        Files.write(output.resolve("metadata.bin"), copy(metadataBytes));
        Files.write(output.resolve("values.bin"), copy(valuesBytes));
    }

    private static void validate(final Path input) throws IOException
    {
        final byte[] metadataFile = Files.readAllBytes(input.resolve("metadata.bin"));
        final byte[] valuesFile = Files.readAllBytes(input.resolve("values.bin"));
        final ByteBuffer metadataBytes = ByteBuffer.allocateDirect(metadataFile.length);
        final ByteBuffer valuesBytes = ByteBuffer.allocateDirect(valuesFile.length);
        metadataBytes.put(metadataFile).clear();
        valuesBytes.put(valuesFile).clear();

        final CountersReader reader =
            new CountersReader(new UnsafeBuffer(metadataBytes), new UnsafeBuffer(valuesBytes), US_ASCII);
        require(4 == reader.maxCounterId() + 1, "capacity");
        require(CountersReader.RECORD_ALLOCATED == reader.getCounterState(0), "allocated state");
        require(17 == reader.getCounterTypeId(0), "type");
        require(142L == reader.getCounterValue(0), "value");
        require(1_101L == reader.getCounterRegistrationId(0), "registration");
        require(2_202L == reader.getCounterOwnerId(0), "owner");
        require(3_303L == reader.getCounterReferenceId(0), "reference");
        require("rust-alpha".equals(reader.getCounterLabel(0)), "label");

        require(CountersReader.RECORD_ALLOCATED == reader.getCounterState(1), "max state");
        require(-15 == reader.getCounterTypeId(1), "max type");
        require(CountersReader.MAX_LABEL_LENGTH == reader.getCounterLabel(1).length(), "max label");
        final int keyOffset = CountersReader.metaDataOffset(1) + CountersReader.KEY_OFFSET;
        for (int index = 0; index < CountersReader.MAX_KEY_LENGTH; index++)
        {
            require((byte)0xA6 == metadataBytes.get(keyOffset + index), "max key");
        }

        require(CountersReader.RECORD_ALLOCATED == reader.getCounterState(2), "reused state");
        require(23 == reader.getCounterTypeId(2), "reused type");
        require(0L == reader.getCounterValue(2), "reused value reset");
        require(0L == reader.getCounterRegistrationId(2), "reused registration reset");
        require(0L == reader.getCounterOwnerId(2), "reused owner reset");
        require(0L == reader.getCounterReferenceId(2), "reused reference reset");
        require("reused".equals(reader.getCounterLabel(2)), "reused label");

        require(CountersReader.RECORD_RECLAIMED == reader.getCounterState(3), "reclaimed state");
        require(2_000L == reader.getFreeForReuseDeadline(3), "reuse deadline");
        final int reclaimedKeyOffset = CountersReader.metaDataOffset(3) + CountersReader.KEY_OFFSET;
        for (int index = 0; index < CountersReader.MAX_KEY_LENGTH; index++)
        {
            require((byte)0 == metadataBytes.get(reclaimedKeyOffset + index), "cleared key");
        }
    }

    private static void require(final boolean condition, final String description)
    {
        if (!condition)
        {
            throw new AssertionError("Rust counter fixture failed Java validation: " + description);
        }
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
