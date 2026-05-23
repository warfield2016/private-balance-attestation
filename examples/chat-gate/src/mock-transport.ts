// In-process MessagingTransport for the chat-gate demo. Two paired
// instances form an in-memory channel: A.send(payload) shows up at
// B.recv() and vice versa. Useful for local demos and tests.
//
// Replace with a real Logos Messaging client when binding is ready.

import type { Hash32, MessagingTransport } from "@lp-0005/sdk";

interface QueuedMsg {
  from: Hash32;
  payload: Uint8Array;
}

export class InProcessTransport implements MessagingTransport {
  private inbox: QueuedMsg[] = [];
  private waiters: ((m: QueuedMsg) => void)[] = [];
  private peer?: InProcessTransport;
  private peerPk?: Hash32;
  private closed = false;

  static pair(aPk: Hash32, bPk: Hash32): [InProcessTransport, InProcessTransport] {
    const a = new InProcessTransport();
    const b = new InProcessTransport();
    a.peer = b;
    a.peerPk = bPk;
    b.peer = a;
    b.peerPk = aPk;
    return [a, b];
  }

  async send(_to: Hash32, payload: Uint8Array): Promise<void> {
    if (this.closed) throw new Error("transport closed");
    if (!this.peer || !this.peerPk) throw new Error("transport has no peer");
    this.peer.deliver({ from: this.peerPk, payload });
  }

  async recv(): Promise<{ from: Hash32; payload: Uint8Array }> {
    if (this.inbox.length) return this.inbox.shift()!;
    return new Promise((resolve) => this.waiters.push(resolve));
  }

  async close(): Promise<void> {
    this.closed = true;
    while (this.waiters.length) {
      const w = this.waiters.shift()!;
      w({ from: new Uint8Array(32), payload: new Uint8Array(0) });
    }
  }

  private deliver(m: QueuedMsg) {
    if (this.waiters.length) {
      const w = this.waiters.shift()!;
      w(m);
      return;
    }
    this.inbox.push(m);
  }
}
