"use client"

import * as React from "react"

type Theme = "light" | "dark" | "system"

type ThemeProviderProps = {
  children: React.ReactNode
  defaultTheme?: Theme
  storageKey?: string
}

type ThemeProviderState = {
  theme: Theme
  setTheme: (theme: Theme) => void
  actualTheme: "light" | "dark"
}

const initialState: ThemeProviderState = {
  theme: "system",
  setTheme: () => null,
  actualTheme: "light",
}

const ThemeProviderContext = React.createContext<ThemeProviderState>(initialState)

export function ThemeProvider({
  children,
  defaultTheme = "system",
  storageKey = "bilibili-down-theme",
  ...props
}: ThemeProviderProps) {
  const [theme, setThemeState] = React.useState<Theme>(
    () => (typeof window !== "undefined" ? (localStorage.getItem(storageKey) as Theme) : null) || defaultTheme
  )
  const [actualTheme, setActualTheme] = React.useState<"light" | "dark">("light")

  React.useEffect(() => {
    const root = window.document.documentElement
    const systemTheme = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light"
    const effectiveTheme = theme === "system" ? systemTheme : theme

    root.classList.remove("light", "dark")
    root.classList.add(effectiveTheme)
    setActualTheme(effectiveTheme)
  }, [theme])

  React.useEffect(() => {
    if (theme === "system") {
      const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)")
      const handleChange = () => {
        const systemTheme = mediaQuery.matches ? "dark" : "light"
        const root = window.document.documentElement
        root.classList.remove("light", "dark")
        root.classList.add(systemTheme)
        setActualTheme(systemTheme)
      }

      mediaQuery.addEventListener("change", handleChange)
      return () => mediaQuery.removeEventListener("change", handleChange)
    }
  }, [theme])

  const value = {
    theme,
    setTheme: (newTheme: Theme) => {
      // Only save to localStorage if user manually sets a theme (not system)
      if (newTheme !== "system") {
        localStorage.setItem(storageKey, newTheme)
      } else {
        // Remove from storage when setting to system
        localStorage.removeItem(storageKey)
      }
      setThemeState(newTheme)
    },
    actualTheme,
  }

  return (
    <ThemeProviderContext.Provider {...props} value={value}>
      {children}
    </ThemeProviderContext.Provider>
  )
}

export const useTheme = () => {
  const context = React.useContext(ThemeProviderContext)

  if (context === undefined) throw new Error("useTheme must be used within a ThemeProvider")

  return context
}
