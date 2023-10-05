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
  id: number;
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
  verifyCredentials: (args: {
    request: VcFlowRequest;
    rpOrigin: string;
  }) => Promise<VcVerifiablePresentation["result"]>;
}) => {
  if (window.opener === null) {
    // If there's no `window.opener` a user has manually navigated to "/vc-flow".
    // Signal that there will never be an authentication request incoming.
    return "orphan";
  }

  // Send a message to indicate we're ready.
  // NOTE: Because `window.opener.origin` cannot be accessed, this message
  // is sent with "*" as the target origin. This is safe as no sensitive
  // information is being communicated here.
  window.opener.postMessage(VcFlowReady, "*");

  onProgress("waiting");

  const { origin, request } = await waitForRequest();
  const reqId = request.id;

  onProgress("verifying");

  const result = await verifyCredentials({ request, rpOrigin: origin });

  window.opener.postMessage(
    {
      id: reqId,
      jsonrpc: "2.0",
      result,
    } satisfies VcVerifiablePresentation,
    origin
  );
};

const waitForRequest = (): Promise<{
  request: VcFlowRequest;
  origin: string;
}> => {
  return new Promise((resolve) => {
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

      resolve({ request: result.data, origin: evnt.origin });
    };

    // Set up an event listener for receiving messages from the client.
    window.addEventListener("message", messageEventHandler);
  });
};
