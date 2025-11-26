import * as React from "react"

export function Footer() {
  return (
    <footer className="w-full border-t border-border bg-background">
      <div className="container mx-auto px-4 py-8">
        <div className="flex flex-col items-center justify-center space-y-4 text-center">
          <div className="text-sm text-muted-foreground">
            <p className="mb-2">
              该项目仅供学习及研究使用，严禁用于违反法律以及哔哩哔哩用户协议的任何用途
            </p>
            <p>
              <a
                href="https://beian.miit.gov.cn/"
                target="_blank"
                rel="noopener noreferrer"
                className="hover:text-foreground transition-colors"
              >
                鲁ICP备2023028944号-3
              </a>
            </p>
          </div>
          <div className="text-xs text-muted-foreground">
            © {new Date().getFullYear()} Bilibili-Down. All rights reserved.
          </div>
        </div>
      </div>
    </footer>
  )
}
