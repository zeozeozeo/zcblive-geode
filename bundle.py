import os
PATH = 'zcblive-build/zcblive.dll'

o = open('src/embed.hpp', 'w', encoding='utf-8')
o.write('#pragma once\n')
o.write('#include <cstdint>\n')
o.write('#include <array>\n')
o.write(f'const std::array<uint8_t, {os.path.getsize(PATH)}> zcblive_dll = {{')

with open(PATH, 'rb') as f:
    while True:
        data = f.read(256)
        if len(data) == 0:
            break
        for b in data:
            o.write(str(b) + ',')
        
o.write('};\n')
o.close()
