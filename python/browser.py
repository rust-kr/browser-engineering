import tkinter

import http


WIDTH, HEIGHT = 800, 600
HSTEP, VSTEP = 13, 18
SCROLL_STEP = 100


class Browser:

    def __init__(self):
        self.window = tkinter.Tk()
        self.scroll = 0
        self.window.bind("<Up>", self.scrollup)
        self.window.bind("<Down>", self.scrolldown)
        self.canvas = tkinter.Canvas(
            self.window,
            width=WIDTH,
            height=HEIGHT
        )
        self.canvas.pack()

    def load(self, url):
        headers, body = http.request(url)
        text = http.lex(body)
        self.display_list = self.layout(text)
        self.render()

    def layout(self, text):
        display_list = []
        cursor_x, cursor_y = HSTEP, VSTEP
        for c in text:
            display_list.append((cursor_x, cursor_y, c))
            cursor_x += HSTEP
            if cursor_x >= WIDTH - HSTEP or c == '\n':
                cursor_y += VSTEP
                cursor_x = HSTEP
        return display_list

    def render(self):
        self.canvas.delete("all")
        for x, y, c in self.display_list:
            if y > self.scroll + HEIGHT:
                continue
            if y + VSTEP < self.scroll:
                continue
            self.canvas.create_text(x, y - self.scroll, text=c)

    def scrolldown(self, e):
        self.scroll += SCROLL_STEP
        self.render()

    def scrollup(self, e):
        self.scroll -= SCROLL_STEP
        self.render()


if __name__ == '__main__':
    import sys
    Browser().load(sys.argv[1])
    tkinter.mainloop()
