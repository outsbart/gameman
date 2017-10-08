"""
Python 3.6
Fetches the json opcodes and turns them into csv
Thanks to https://github.com/Prehistoricman for the json file
"""

import json
from urllib import request
from pprint import pprint

def load_ops():
	response = request.urlopen('https://raw.githubusercontent.com/Prehistoricman/GBEmulatorStuff/master/GameBoyOpcodes.json')
	return json.load(response)

operations = load_ops()

for op_type in ("unprefixed", "cbprefixed"):
	with open(f'{op_type}.csv', 'w') as outfile:
		outfile.write("code,mnemonic,operand1,operand2,flag_z,flag_h,flag_n,flag_c,cycles_ok,cycles_no")
		for op_id, op in operations[op_type].items():

			outfile.write("\n")

			code = op_id,
			mnemonic = op["mnemonic"]
			flag_z = op["flags_ZHNC"][0]
			if flag_z == '-':
				flag_z = ''
			flag_h = op["flags_ZHNC"][1]
			if flag_h == '-':
				flag_h = ''
			flag_n = op["flags_ZHNC"][2]
			if flag_n == '-':
				flag_n = ''
			flag_c = op["flags_ZHNC"][3]
			if flag_c == '-':
				flag_c = ''

			operand1 = op.get('operand1','')
			operand2 = op.get('operand2','')
			cycles_ok = op["cycles"][0]
			cycles_no = ''

			if len(op["cycles"]) > 1:
				cycles_no = op["cycles"][1]

			outfile.write(f"{op_id},{mnemonic},{operand1},{operand2},{flag_z},{flag_h},{flag_n},{flag_c},{cycles_ok},{cycles_no}")

