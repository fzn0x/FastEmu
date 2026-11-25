//! Bytes a native reads from a game's stream, counted on the heap while
//! they grow.
//!
//! `Image.createImage(InputStream)` and `Manager.createPlayer(InputStream,
//! String)` read a stream to its end through the game's own `read`. The
//! game's code runs between the reads, and so does every other thread of
//! the game, so bytes kept in a host vector the heap does not see add up
//! over the threads: eight threads that each read an image stream held
//! 128 MiB beside a heap of 2 MiB. Here the bytes grow in the payload of
//! a plain `java/lang/Object` that the native roots, which the heap counts
//! at the room of its vector. The heap is asked before the vector grows,
//! so threads that read at once share the heap's room, and a stream the
//! heap cannot hold ends in `OutOfMemoryError`.

use fastemu_jvm::{Ctx, JResult, ObjRef, Payload};

/// The bytes read so far.
struct StreamBytes(Vec<u8>);

impl Payload for StreamBytes {
    fn heap_bytes(&self) -> usize {
        self.0.capacity()
    }
}

/// A holder of the bytes read from a stream: a plain object with a
/// [`StreamBytes`] payload, rooted in the native's locals until it
/// returns, and garbage after that.
#[derive(Clone, Copy)]
pub(crate) struct CountedBytes(ObjRef);

impl CountedBytes {
    /// An empty holder.
    pub(crate) fn new(ctx: &mut Ctx<'_>) -> JResult<Self> {
        let class = ctx.find_class("java/lang/Object")?;
        let holder = ctx.new_object(class)?;
        ctx.set_payload(holder, StreamBytes(Vec::new()));
        Ok(Self(holder))
    }

    /// How many bytes it holds.
    pub(crate) fn len(self, ctx: &Ctx<'_>) -> JResult<usize> {
        Ok(ctx.payload::<StreamBytes>(self.0)?.0.len())
    }

    /// Adds `bytes`. When they do not fit, the vector grows, doubled as a
    /// vector grows, but never past `max` or the whole heap unless the
    /// bytes need it. The heap is asked for the grown vector first, while
    /// it still counts the old one, as both exist while the bytes move:
    /// `OutOfMemoryError` when it cannot hold it, with nothing added.
    pub(crate) fn push(self, ctx: &mut Ctx<'_>, bytes: &[u8], max: usize) -> JResult<()> {
        let (len, room) = {
            let held = &ctx.payload::<StreamBytes>(self.0)?.0;
            (held.len(), held.capacity())
        };
        let needed = len.saturating_add(bytes.len());
        let grows = needed > room;
        if grows {
            let (heap, _) = ctx.heap_usage();
            let grown = room.saturating_mul(2).min(max).min(heap).max(needed);
            ctx.check_alloc(grown)?;
            ctx.payload_mut::<StreamBytes>(self.0)?
                .0
                .reserve_exact(grown - len);
        }
        ctx.payload_mut::<StreamBytes>(self.0)?
            .0
            .extend_from_slice(bytes);
        if grows {
            ctx.recount(self.0);
        }
        Ok(())
    }

    /// Takes the bytes out and leaves the holder empty. The heap goes on
    /// counting them, at the room they had, until it collects the holder
    /// after the native returns: they are still held, by whatever the
    /// native does with them next, while other threads run.
    pub(crate) fn take(self, ctx: &mut Ctx<'_>) -> JResult<Vec<u8>> {
        Ok(std::mem::take(
            &mut ctx.payload_mut::<StreamBytes>(self.0)?.0,
        ))
    }

    /// Takes the bytes out and stops counting them, for a native that
    /// hands them to an object the heap counts (asking the heap for it
    /// first) before it calls Java or releases the VM lock. No other
    /// thread of the game runs before then, so they never add up.
    pub(crate) fn take_uncounted(self, ctx: &mut Ctx<'_>) -> JResult<Vec<u8>> {
        let bytes = self.take(ctx)?;
        ctx.recount(self.0);
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use fastemu_jvm::{JResult, MemorySource, Vm, VmConfig};

    use super::CountedBytes;

    fn vm(heap: usize) -> Vm {
        let config = VmConfig {
            heap_bytes: heap,
            log: Arc::new(|_| {}),
            ..VmConfig::default()
        };
        Vm::new(config, Arc::new(MemorySource::new()), Vec::new())
    }

    #[test]
    fn the_bytes_held_count_on_the_heap_and_end_at_its_size() {
        let vm = vm(1 << 20);
        let outcome = vm.attach("reader", |ctx| -> JResult<()> {
            let held = CountedBytes::new(ctx)?;
            let (_, before) = ctx.heap_usage();
            held.push(ctx, &[7; 4096], 16 << 20)?;
            held.push(ctx, &[7; 4096], 16 << 20)?;
            // Counted at the room of the vector, doubled when it grew.
            let (_, after) = ctx.heap_usage();
            assert_eq!(after - before, 8192);
            assert_eq!(held.len(ctx)?, 8192);
            // A heap of 1 MiB holds 1 MiB and refuses the next growth.
            let mut pushed = 2;
            while held.push(ctx, &[7; 4096], 16 << 20).is_ok() {
                pushed += 1;
            }
            assert_eq!(pushed, 256);
            assert_eq!(held.len(ctx)?, 1 << 20);
            // Taken out, still counted; taken out uncounted, not.
            let bytes = held.take(ctx)?;
            assert_eq!(bytes.len(), 1 << 20);
            let (_, taken) = ctx.heap_usage();
            assert!(taken - before >= 1 << 20);
            held.push(ctx, &[1; 10], 16 << 20)?;
            assert_eq!(held.take_uncounted(ctx)?, [1; 10]);
            let (_, uncounted) = ctx.heap_usage();
            assert!(uncounted - before < 4096, "{}", uncounted - before);
            Ok(())
        });
        assert!(outcome.is_ok());
    }
}
