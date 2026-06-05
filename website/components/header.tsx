"use client"

import * as React from "react"
import Link from "next/link"
import Image from "next/image"
import { Github } from "lucide-react"
import { Button } from "@/components/ui/button"
import { ThemeToggle } from "@/components/theme-toggle"

export function Header() {
  return (
    <header className="sticky top-0 z-50 w-full border-b border-border bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="container mx-auto flex h-16 items-center justify-between px-4">
        <Link href="/" className="flex items-center space-x-2">
          <Image src="/favicon.ico" alt="Bilibili-Down Logo" width={32} height={32} />
          <span className="text-xl font-bold">Bilibili-Down</span>
        </Link>

        <div className="flex items-center space-x-4">
          <Button
            variant="ghost"
            size="sm"
            render={
              <a
                href="https://github.com/NORMAL-EX/Bilibili-Down"
                target="_blank"
                rel="noopener noreferrer"
              />
            }
            className="flex items-center space-x-2"
          >
            <Github className="h-5 w-5" />
            <span className="hidden sm:inline">GitHub</span>
          </Button>
          <ThemeToggle />
        </div>
      </div>
    </header>
  )
}
