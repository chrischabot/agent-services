import { createHash } from "node:crypto";

export const DEFAULT_EMAIL_POLICY = Object.freeze({
  provider: "cloudflare_email_routing",
  maxBodyBytes: 32768,
  maxPreviewChars: 4000,
  maxAttachments: 8,
  maxAttachmentBytes: 2 * 1024 * 1024,
  maxTotalAttachmentBytes: 4 * 1024 * 1024,
  stageAttachments: false,
  requireDmarcPass: true,
  maxAgeSeconds: 24 * 60 * 60,
  allowedSenders: [],
  routes: []
});

export function mapInboundEmail(input, policy = {}) {
  const cfg = mergePolicy(policy);
  const warnings = [];
  const messageId = normalizeMessageId(input.messageId ?? headerValue(input.headers, "message-id"));
  if (!messageId) return rejected("missing_message_id", warnings);
  if (hasExistingMessageId(cfg.existingMessageIds, messageId)) {
    return {
      accepted: false,
      duplicate: true,
      reason: "duplicate_message_id",
      idempotencyKey: emailIdempotencyKey(messageId),
      warnings
    };
  }

  const autoReply = autoResponderReason(input);
  if (autoReply) return rejected(autoReply, warnings);

  const bodyBytes = byteLength(input.bodyText ?? "") + byteLength(input.bodyHtml ?? "");
  if (bodyBytes > cfg.maxBodyBytes) return rejected("oversized_body", warnings, { bodyBytes });

  const attachmentDecision = evaluateAttachments(input.attachments ?? [], cfg);
  warnings.push(...attachmentDecision.warnings);
  if (!attachmentDecision.accepted) {
    return rejected("attachment_policy_rejected", warnings, attachmentDecision.metadata);
  }

  const recipient = normalizeEmailAddress(input.envelopeTo ?? input.recipient ?? input.to);
  if (!recipient) return rejected("missing_envelope_recipient", warnings);
  const route = findRoute(cfg.routes, recipient);
  if (!route) return rejected("unauthorized_route", warnings, { recipient });

  const envelopeFrom = normalizeEmailAddress(input.envelopeFrom ?? input.mailFrom);
  const signedSender = normalizeEmailAddress(input.signedSender ?? input.auth?.signedSender);
  const headerFrom = normalizeEmailAddress(input.headerFrom ?? headerValue(input.headers, "from"));
  const trustedSender = signedSender ?? envelopeFrom;
  if (!trustedSender) return rejected("missing_trusted_sender", warnings);
  if (headerFrom && headerFrom !== trustedSender) warnings.push("header_from_is_display_only_and_differs_from_trusted_sender");

  const auth = normalizeAuth(input.auth);
  if (cfg.requireDmarcPass && auth.dmarc !== "pass") {
    return rejected("sender_authentication_failed", warnings, { trustedSender, auth });
  }

  const senderRule = findSenderRule(cfg.allowedSenders, trustedSender, route.id);
  if (!senderRule) return rejected("unauthorized_sender", warnings, { trustedSender, recipient, routeId: route.id });

  const rawEvidenceText = chooseEvidenceText(input);
  const sanitizedText = sanitizeEvidenceText(rawEvidenceText).slice(0, cfg.maxPreviewChars);
  const trackingLinks = findTrackingLinks(rawEvidenceText);
  if (trackingLinks.length > 0) warnings.push("tracking_links_preserved_as_unfetched_evidence");

  const idempotencyKey = emailIdempotencyKey(messageId);
  const receivedAt = input.receivedAt ?? new Date(0).toISOString();
  const subject = safeSubject(input.subject ?? headerValue(input.headers, "subject") ?? "(no subject)");
  const provenance = {
    provider: cfg.provider,
    messageId,
    receivedAt,
    trustedSender,
    envelopeFrom,
    signedSender,
    headerFrom,
    recipient,
    auth,
    routeId: route.id
  };

  const sourceCard = route.createSourceCard === false ? null : {
    type: "source_card",
    sourceType: "email",
    title: `Email: ${subject}`,
    canonicalUrl: `email:${idempotencyKey}`,
    summary: summarize(sanitizedText),
    trust: "untrusted_email_evidence",
    bodyInstructionPolicy: "email_body_is_evidence_never_instructions",
    agentInstructions: [],
    provenance,
    warnings,
    trackingLinks,
    attachments: attachmentDecision.metadata.attachments
  };

  const channelMessage = {
    type: "channel_message",
    channel: "email",
    direction: "inbound",
    subject: `email:${trustedSender}`,
    projectId: route.projectId ?? null,
    sourceEventId: idempotencyKey,
    text: sanitizedText,
    trust: "UNTRUSTED_CHANNEL_EVIDENCE",
    metadata: {
      messageId,
      recipient,
      trustedSender,
      headerFrom,
      subject,
      routeId: route.id,
      warnings
    }
  };

  return {
    accepted: true,
    duplicate: false,
    idempotencyKey,
    edgeEvent: {
      source: "email",
      idempotencyKey,
      maxAgeSeconds: cfg.maxAgeSeconds,
      payload: {
        provider: cfg.provider,
        messageId,
        receivedAt,
        routeId: route.id,
        trustedSender,
        recipient,
        subject,
        sanitizedText,
        warnings,
        trackingLinks,
        attachments: attachmentDecision.metadata.attachments
      }
    },
    channelMessage,
    sourceCard,
    warnings
  };
}

function mergePolicy(policy) {
  return {
    ...DEFAULT_EMAIL_POLICY,
    ...policy,
    allowedSenders: policy.allowedSenders ?? DEFAULT_EMAIL_POLICY.allowedSenders,
    routes: policy.routes ?? DEFAULT_EMAIL_POLICY.routes
  };
}

function rejected(reason, warnings = [], metadata = {}) {
  return { accepted: false, duplicate: false, reason, warnings, metadata };
}

function normalizeMessageId(value) {
  if (typeof value !== "string") return null;
  const trimmed = value.trim().replace(/^<|>$/g, "");
  if (!trimmed || trimmed.length > 300 || /[\r\n]/.test(trimmed)) return null;
  return trimmed.toLowerCase();
}

function emailIdempotencyKey(messageId) {
  return `email:message:${createHash("sha256").update(messageId).digest("hex").slice(0, 32)}`;
}

function hasExistingMessageId(existing, messageId) {
  if (!existing) return false;
  if (existing instanceof Set) return existing.has(messageId);
  if (Array.isArray(existing)) return existing.map(normalizeMessageId).includes(messageId);
  return false;
}

function normalizeEmailAddress(value) {
  if (typeof value !== "string") return null;
  const trimmed = value.trim();
  const bracketed = trimmed.match(/<([^<>]+)>/);
  const candidate = (bracketed ? bracketed[1] : trimmed).trim().replace(/^mailto:/i, "");
  if (!candidate || candidate.length > 320 || /[\r\n]/.test(candidate)) return null;
  const match = candidate.match(/^([A-Z0-9._%+\-']+)@([A-Z0-9.-]+\.[A-Z]{2,})$/i);
  if (!match) return null;
  return `${match[1].toLowerCase()}@${match[2].toLowerCase()}`;
}

function headerValue(headers, name) {
  if (!headers || typeof headers !== "object") return null;
  const wanted = name.toLowerCase();
  for (const [key, value] of Object.entries(headers)) {
    if (key.toLowerCase() === wanted && typeof value === "string") return value;
  }
  return null;
}

function normalizeAuth(auth = {}) {
  return {
    spf: normalizeVerdict(auth.spf),
    dkim: normalizeVerdict(auth.dkim),
    dmarc: normalizeVerdict(auth.dmarc)
  };
}

function normalizeVerdict(value) {
  return typeof value === "string" ? value.trim().toLowerCase() : "unknown";
}

function findRoute(routes, recipient) {
  return routes.find((route) => normalizeEmailAddress(route.recipient) === recipient) ?? null;
}

function findSenderRule(rules, trustedSender, routeId) {
  return rules.find((rule) => {
    const address = normalizeEmailAddress(rule.address);
    const domain = typeof rule.domain === "string" ? rule.domain.toLowerCase() : null;
    const senderDomain = trustedSender.split("@")[1];
    const senderMatches = address === trustedSender || domain === senderDomain;
    if (!senderMatches) return false;
    const allowedRoutes = rule.routes ?? ["*"];
    return allowedRoutes.includes("*") || allowedRoutes.includes(routeId);
  }) ?? null;
}

function autoResponderReason(input) {
  const autoSubmitted = String(input.autoSubmitted ?? headerValue(input.headers, "auto-submitted") ?? "").toLowerCase();
  const precedence = String(input.precedence ?? headerValue(input.headers, "precedence") ?? "").toLowerCase();
  if (autoSubmitted && autoSubmitted !== "no") return "auto_responder_ignored";
  if (["bulk", "junk", "list", "auto_reply"].includes(precedence)) return "auto_responder_ignored";
  return null;
}

function evaluateAttachments(attachments, cfg) {
  const metadata = {
    policy: cfg.stageAttachments ? "stage_metadata_only" : "ignore_content_store_metadata_only",
    attachments: []
  };
  const warnings = [];
  if (!Array.isArray(attachments) || attachments.length === 0) return { accepted: true, warnings, metadata };
  if (attachments.length > cfg.maxAttachments) {
    return { accepted: false, warnings, metadata: { ...metadata, count: attachments.length } };
  }
  let total = 0;
  for (const attachment of attachments) {
    const sizeBytes = Number(attachment.sizeBytes ?? 0);
    if (!Number.isFinite(sizeBytes) || sizeBytes < 0) return { accepted: false, warnings, metadata };
    total += sizeBytes;
    metadata.attachments.push({
      filename: typeof attachment.filename === "string" ? attachment.filename.slice(0, 200) : null,
      contentType: typeof attachment.contentType === "string" ? attachment.contentType.slice(0, 120) : "application/octet-stream",
      sizeBytes,
      disposition: cfg.stageAttachments ? "staged_pending_review" : "ignored"
    });
    if (sizeBytes > cfg.maxAttachmentBytes) return { accepted: false, warnings, metadata: { ...metadata, totalBytes: total } };
  }
  if (total > cfg.maxTotalAttachmentBytes) return { accepted: false, warnings, metadata: { ...metadata, totalBytes: total } };
  warnings.push(cfg.stageAttachments ? "attachments_staged_pending_review" : "attachments_ignored_by_policy");
  metadata.totalBytes = total;
  return { accepted: true, warnings, metadata };
}

function chooseEvidenceText(input) {
  if (typeof input.bodyText === "string" && input.bodyText.trim().length > 0) return input.bodyText;
  if (typeof input.bodyHtml === "string") return htmlToText(input.bodyHtml);
  return "";
}

function sanitizeEvidenceText(value) {
  return String(value)
    .replace(/\u0000/g, "")
    .replace(/\r\n/g, "\n")
    .replace(/[ \t]+\n/g, "\n")
    .replace(/\n{4,}/g, "\n\n\n")
    .trim();
}

function htmlToText(html) {
  return String(html)
    .replace(/<!--[\s\S]*?-->/g, " ")
    .replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, " ")
    .replace(/<style\b[^>]*>[\s\S]*?<\/style>/gi, " ")
    .replace(/<[^>]+>/g, " ")
    .replace(/&nbsp;/gi, " ")
    .replace(/&amp;/gi, "&")
    .replace(/&lt;/gi, "<")
    .replace(/&gt;/gi, ">")
    .replace(/&quot;/gi, "\"")
    .replace(/&#39;/gi, "'");
}

function findTrackingLinks(value) {
  const links = String(value).match(/https?:\/\/[^\s"'<>]+/gi) ?? [];
  return links
    .filter((link) => /[?&](utm_[^=]+|fbclid|gclid|mc_cid|mc_eid)=/i.test(link) || /\/(track|tracking|open|click)\b/i.test(link))
    .slice(0, 20);
}

function safeSubject(value) {
  return String(value).replace(/[\r\n]+/g, " ").trim().slice(0, 180) || "(no subject)";
}

function summarize(value) {
  return sanitizeEvidenceText(value).slice(0, 500);
}

function byteLength(value) {
  return Buffer.byteLength(String(value), "utf8");
}
