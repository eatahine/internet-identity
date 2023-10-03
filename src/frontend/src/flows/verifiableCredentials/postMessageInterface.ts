import { toast } from "$src/components/toast";
import { z } from "zod";

export const VcFlowReady = {
  jsonrpc: "2.0",
  method: "vc-flow-ready",
};

// https://www.jsonrpc.org/specification
// https://github.com/dfinity/internet-identity/blob/vc-mvp/docs/vc-spec.md#identity-provider-api
export const VcFlowRequest = z.object({
  id: z.number() /* TODO: cannot contain fraction */,
  jsonrpc: z.literal("2.0"),
  method: z.literal("request_credential"),
  params: z.object({
    issuer: z.object({
      issuerOrigin: z.string() /* TODO: should be a URL */,
      credentialId: z.string(),
    }),
    credentialSubject: z.string() /* TODO: should be a principal */,
  }),
});

export type VcFlowRequest = z.infer<typeof VcFlowRequest>;

export type VcVerifiablePresentation = {
  id: string;
  jsonrpc: "2.0";
  result: {
    verifiablePresentation: string;
  };
};

export const vcProtocol = async ({
  onProgress,
  verifyCredentials,
}: {
  onProgress: (state: "waiting" | "verifying") => void;
  verifyCredentials: (
    request: VcFlowRequest
  ) => Promise<VcVerifiablePresentation>;
}) => {
  if (window.opener === null) {
    // If there's no `window.opener` a user has manually navigated to "/vc-flow".
    // Signal that there will never be an authentication request incoming.
    return "orphan";
  }

  window.opener.postMessage(VcFlowReady);

  onProgress("waiting");

  const request = await waitForRequest();

  onProgress("verifying");

  const response = await verifyCredentials(request);

  window.opener.postMessage(response satisfies VcVerifiablePresentation);
};

const waitForRequest = (): Promise<VcFlowRequest> => {
  return new Promise<VcFlowRequest>((resolve) => {
    const messageEventHandler = (evnt: MessageEvent) => {
      const message: unknown = evnt.data;
      const result = VcFlowRequest.safeParse(message);

      if (!result.success) {
        const message = `Unexpected error: flow request ` + result.error;
        console.error(message);
        toast.error(message);
        return; // XXX: this just waits further; correct?
      }

      window.removeEventListener("message", messageEventHandler);

      resolve(result.data);
    };

    // Set up an event listener for receiving messages from the client.
    window.addEventListener("message", messageEventHandler);
  });
};
