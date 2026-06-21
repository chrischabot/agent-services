import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { mapInboundEmail } from "../src/email-mapper.mjs";

const policy = {
  maxBodyBytes: 1024,
  allowedSenders: [
    { address: "founder@example.com", routes: ["launches"] },
    { domain: "alerts.example.com", routes: ["alerts"] }
  ],
  routes: [
    { id: "launches", recipient: "launches@arcwell.test", projectId: "project-launches", createSourceCard: true },
    { id: "alerts", recipient: "alerts@arcwell.test", projectId: "project-alerts", createSourceCard: true }
  ],
  existingMessageIds: ["launch-1@example.com"]
};

test("CLAIM: trusted email metadata maps to source cards/channel messages while hostile bodies remain evidence", async (t) => {
  // PRECONDITIONS: a Cloudflare Email Routing adapter has supplied envelope/signed sender metadata.
  // POSTCONDITIONS: only authorized routes/senders map; body text is never emitted as instructions.
  // ORACLE: policy matrix plus output trust labels and empty agentInstructions.
  // SEVERITY: Severe because email is attacker-controlled and can target downstream agents.
  const fixtures = JSON.parse(await readFile(new URL("../fixtures/adversarial-cases.json", import.meta.url), "utf8"));
  for (const fixture of fixtures) {
    await t.test(fixture.name, () => {
      const input = expandFixture(fixture.input);
      const result = mapInboundEmail(input, policy);
      assert.equal(result.accepted, fixture.expect.accepted);
      if ("duplicate" in fixture.expect) assert.equal(result.duplicate, fixture.expect.duplicate);
      if (fixture.expect.reason) assert.equal(result.reason, fixture.expect.reason);

      if (result.accepted) {
        assert.equal(result.sourceCard?.trust, fixture.expect.sourceCardTrust ?? "untrusted_email_evidence");
        assert.equal(result.channelMessage.trust, fixture.expect.channelTrust ?? "UNTRUSTED_CHANNEL_EVIDENCE");
        assert.deepEqual(result.sourceCard.agentInstructions, []);
        assert.equal(result.sourceCard.bodyInstructionPolicy, "email_body_is_evidence_never_instructions");
        assert.equal(result.edgeEvent.source, "email");
        assert.match(result.idempotencyKey, /^email:message:[a-f0-9]{32}$/);
      }

      for (const snippet of fixture.expect.contains ?? []) {
        assert.match(result.channelMessage.text, new RegExp(escapeRegExp(snippet)));
      }
      for (const snippet of fixture.expect.notContains ?? []) {
        assert.doesNotMatch(result.channelMessage.text, new RegExp(escapeRegExp(snippet), "i"));
      }
      if ("trackingLinks" in fixture.expect) {
        assert.equal(result.sourceCard.trackingLinks.length, fixture.expect.trackingLinks);
        assert.equal(result.warnings.includes("tracking_links_preserved_as_unfetched_evidence"), true);
      }
    });
  }
});

test("spoofed From header cannot authorize an untrusted envelope sender even when display name looks safe", () => {
  const result = mapInboundEmail(
    {
      messageId: "<spoof-dmarc-pass@evil.test>",
      envelopeFrom: "attacker@evil.test",
      signedSender: "attacker@evil.test",
      headerFrom: "Founder <founder@example.com>",
      envelopeTo: "launches@arcwell.test",
      subject: "display spoof",
      auth: { spf: "pass", dkim: "pass", dmarc: "pass" },
      bodyText: "The header looks authorized, but the trusted sender is not."
    },
    policy
  );
  assert.equal(result.accepted, false);
  assert.equal(result.reason, "unauthorized_sender");
  assert.equal(result.metadata.trustedSender, "attacker@evil.test");
});

function expandFixture(input) {
  if (input.bodyText === "__OVERSIZED__") {
    return { ...input, bodyText: "x".repeat(2048) };
  }
  return input;
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
