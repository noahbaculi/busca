from PDFNetPython3 import PDFDoc, Optimizer, SDFDoc
from os import path
from glob import glob

while True:  # repeat for batch processing

    # the base directory where PDF files will be searched
    dir = input('Input path: ')

    # check that the directory is valid
    if not path.isdir(dir):
        raise ValueError('The path input is not a valid path.')

    # search for PDF files recursively or only in base directory
    pdf_files = glob(path.join(dir, '**/*.pdf'), recursive=True)  # recursively
    # pdf_files = glob(path.join(dir, "*.{}".format('pdf')))  # only in the base directory

    # check if there are PDF files
    if not pdf_files:
        raise ValueError(f'No PDF files were found in {dir}')

    # loop through PDF file path names
    for pdf_path in pdf_files:
        # compress PDF file with default settins
        doc = PDFDoc(pdf_path)
        doc.InitSecurityHandler()
        Optimizer.Optimize(doc)
        doc.Save(pdf_path, SDFDoc.e_linearized)
        doc.Close()

        print(f'Compressing {pdf_path}...')

    print('Completed.\n\n')