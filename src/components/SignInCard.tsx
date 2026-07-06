import { Banner, Button, ClipboardText, LayerCard, Loader, Text } from "@cloudflare/kumo";
import { SignIn } from "@phosphor-icons/react";
import type { AuthStatus } from "../lib/types";

interface Props {
  auth: AuthStatus;
  onLogin: () => void;
}

export function SignInCard({ auth, onLogin }: Props) {
  const { phase } = auth;
  const awaiting = phase === "awaiting_user" || phase === "polling";

  return (
    <div className="flex h-full flex-col bg-kumo-canvas">
      <header className="flex shrink-0 items-center gap-2.5 px-5 py-4">
        <div className="flex size-7 items-center justify-center rounded-lg bg-kumo-brand text-[15px] font-bold text-white">
          c
        </div>
        <Text variant="heading3" as="span">
          Cohub Desktop
        </Text>
      </header>

      <div className="flex flex-1 items-center justify-center p-5">
        <LayerCard className="w-full rounded-xl p-6">
          {phase === "restoring" ? (
            <div className="flex flex-col items-center gap-3 py-6 text-center">
              <Loader size={28} />
              <Text variant="heading3" as="h2">
                恢复会话中…
              </Text>
              <Text variant="secondary" size="sm">
                正在用本地登录态连接 Cohub。
              </Text>
            </div>
          ) : (
            <>
              <Text variant="heading2" as="h1">
                登录 Cohub
              </Text>

              {phase === "error" && auth.error && (
                <div className="mt-4">
                  <Banner variant="error" title="登录失败" description={auth.error} />
                </div>
              )}

              {awaiting ? (
                <div className="mt-5 flex flex-col gap-4">
                  <Text variant="secondary" size="sm">
                    已在浏览器打开授权页，完成授权后自动连接。
                  </Text>
                  {auth.user_code && (
                    <div className="flex flex-col gap-1.5">
                      <Text variant="secondary" size="xs" as="span">
                        用户码
                      </Text>
                      <ClipboardText text={auth.user_code} size="lg" />
                    </div>
                  )}
                  {auth.verification_uri && (
                    <div className="flex flex-col gap-1.5">
                      <Text variant="secondary" size="xs" as="span">
                        授权链接
                      </Text>
                      <ClipboardText text={auth.verification_uri} size="sm" />
                    </div>
                  )}
                  <div className="flex items-center gap-2 text-kumo-subtle">
                    <Loader size={16} />
                    <Text variant="secondary" size="sm">
                      等待授权完成…
                    </Text>
                  </div>
                </div>
              ) : (
                <div className="mt-5 flex flex-col gap-4">
                  <Text variant="secondary" size="sm">
                    点击后在浏览器打开授权页，完成即可。无需手动粘贴 token。
                  </Text>
                  <Button
                    variant="primary"
                    size="lg"
                    icon={<SignIn size={18} weight="regular" />}
                    loading={phase === "requesting_device"}
                    onClick={onLogin}
                  >
                    开始登录
                  </Button>
                </div>
              )}
            </>
          )}
        </LayerCard>
      </div>
    </div>
  );
}
